mod kind;
mod port;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kind::AppKind;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::watch;

use crate::error::{Error, Result};
use crate::feedback::{Notifier, ToastPart, ToastTone};
use crate::harness_config::WorkspaceRunCommand;
use crate::model::WorkspaceLifecycle;
use crate::process::{LogKind, ProcessSupervisor, SessionEvents, SessionId, ShellCommand};
use crate::store::Store;
use crate::workspaces::{WorkspaceRecord, Workspaces};
use port::{detect_port, serves_https};

/// An app that hasn't printed its port by now counts as crashed.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceAppStatus {
    Stopped,
    Starting,
    Running,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAppRuntime {
    pub workspace_id: String,
    pub status: WorkspaceAppStatus,
    pub port: Option<u16>,
    /// The server announced an `https://` URL, so links to it use that scheme.
    pub https: bool,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceAppControlAction {
    Run,
    Restart,
    Stop,
    Clear,
}

/// Why Run can't start a workspace's app now, or `None` when it can. The one rule: the snapshot
/// carries it for Run, ⌘R and the palette, and `control` enforces it.
pub fn run_block(status: WorkspaceLifecycle, has_run_command: bool) -> Option<&'static str> {
    match status {
        WorkspaceLifecycle::Provisioning => Some("Wait for setup to finish to run the app."),
        WorkspaceLifecycle::ProvisioningFailed => {
            Some("Setup failed. Retry or skip it to run the app.")
        }
        _ if !has_run_command => Some("No run command for this workspace."),
        _ => None,
    }
}

/// A workspace whose app Run can't start now, and why, for the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRunBlock {
    pub workspace_id: String,
    pub reason: String,
}

pub fn run_blocks(
    workspaces: &[WorkspaceRecord],
    run_commands: &[WorkspaceRunCommand],
) -> Vec<WorkspaceRunBlock> {
    workspaces
        .iter()
        .filter_map(|workspace| {
            let has_run_command = run_commands
                .iter()
                .any(|run| run.workspace_id == workspace.id);
            run_block(workspace.status, has_run_command).map(|reason| WorkspaceRunBlock {
                workspace_id: workspace.id.clone(),
                reason: reason.into(),
            })
        })
        .collect()
}

struct WorkspaceAppRecord {
    status: WorkspaceAppStatus,
    port: Option<u16>,
    https: bool,
    exit_code: Option<i32>,
    /// Bumped on every run so a monitor from an earlier run knows to exit.
    generation: u64,
}

#[derive(Clone)]
pub struct WorkspaceAppService {
    process: ProcessSupervisor,
    workspaces: Workspaces,
    store: Store,
    out: Notifier,
    inner: Arc<Mutex<HashMap<String, WorkspaceAppRecord>>>,
    /// Bumped with any workspace's generation, so monitors notice they've been replaced.
    runs: Arc<watch::Sender<u64>>,
}

impl WorkspaceAppService {
    pub fn new(
        process: ProcessSupervisor,
        workspaces: Workspaces,
        store: Store,
        out: Notifier,
    ) -> Self {
        Self {
            process,
            workspaces,
            store,
            out,
            inner: Arc::new(Mutex::new(HashMap::new())),
            runs: Arc::new(watch::Sender::new(0)),
        }
    }

    pub fn snapshot(&self) -> Vec<WorkspaceAppRuntime> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .map(|(workspace_id, record)| WorkspaceAppRuntime {
                workspace_id: workspace_id.clone(),
                status: record.status,
                port: record.port,
                https: record.https,
                exit_code: record.exit_code,
            })
            .collect()
    }

    pub fn runtime(&self, workspace_id: &str) -> WorkspaceAppRuntime {
        let inner = self.inner.lock().unwrap();
        inner
            .get(workspace_id)
            .map(|record| WorkspaceAppRuntime {
                workspace_id: workspace_id.to_string(),
                status: record.status,
                port: record.port,
                https: record.https,
                exit_code: record.exit_code,
            })
            .unwrap_or(WorkspaceAppRuntime {
                workspace_id: workspace_id.to_string(),
                status: WorkspaceAppStatus::Stopped,
                port: None,
                https: false,
                exit_code: None,
            })
    }

    pub async fn control(
        &self,
        workspace_id: &str,
        action: WorkspaceAppControlAction,
    ) -> Result<()> {
        match action {
            WorkspaceAppControlAction::Clear => {
                self.process.clear_log(workspace_id);
                self.out.changed();
                Ok(())
            }
            WorkspaceAppControlAction::Stop => self.stop(workspace_id).await,
            WorkspaceAppControlAction::Restart => self.restart(workspace_id).await,
            WorkspaceAppControlAction::Run => {
                let workspace = self.workspaces.get(workspace_id).await?;
                let has_run_command = self.run_command(&workspace)?.is_some();
                if let Some(reason) = run_block(workspace.status, has_run_command) {
                    return Err(Error::Workspace(reason.into()));
                }
                self.run(workspace_id).await
            }
        }
    }

    async fn run(&self, workspace_id: &str) -> Result<()> {
        let workspace = self.workspaces.get(workspace_id).await?;
        let run_command = self
            .run_command(&workspace)?
            .ok_or_else(|| Error::Process("no run command configured".into()))?;
        let kind = AppKind::detect(&run_command);
        self.stop_other_instances(workspace_id).await?;
        let port = self.pick_port_with_log(workspace_id, kind);
        let mut env = workspace.harness_env().vars();
        env.push(("PORT".into(), port.to_string()));

        let generation = self.next_generation(workspace_id);
        self.set_status(workspace_id, WorkspaceAppStatus::Starting, Some(port), None);
        self.process
            .log(workspace_id, LogKind::Command, run_command.as_str());

        let command = ShellCommand {
            cmd: run_command,
            cwd: workspace.worktree(),
            env,
        };
        let events = self
            .process
            .spawn_shell(run_session(workspace_id), &command, Some(workspace_id))
            .await?;

        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            service.monitor(workspace, events, generation).await;
        });

        self.out.changed();
        Ok(())
    }

    async fn restart(&self, workspace_id: &str) -> Result<()> {
        let name = self.workspace_name(workspace_id).await;
        self.out.toast_parts(
            ToastTone::Default,
            vec![
                ToastPart::Text {
                    value: "Restarting the app in ".into(),
                },
                ToastPart::Code { value: name },
            ],
            Some(workspace_id.to_string()),
        );
        self.halt(workspace_id);
        self.process
            .log(workspace_id, LogKind::Marker, "Restarting");
        self.run(workspace_id).await
    }

    async fn stop(&self, workspace_id: &str) -> Result<()> {
        self.halt(workspace_id);
        self.process
            .log(workspace_id, LogKind::Marker, "App stopped");
        let name = self.workspace_name(workspace_id).await;
        self.out.toast_parts(
            ToastTone::Default,
            vec![
                ToastPart::Text {
                    value: "Stopped the app in ".into(),
                },
                ToastPart::Code { value: name },
            ],
            Some(workspace_id.to_string()),
        );
        self.out.changed();
        Ok(())
    }

    /// Ends the app's session and marks it stopped, without telling the user.
    fn halt(&self, workspace_id: &str) {
        if let Err(error) = self.process.stop_session(&run_session(workspace_id)) {
            log::warn!("stopping the app in {workspace_id}: {error}");
        }
        self.process.log(workspace_id, LogKind::Interrupt, "^C");
        self.set_status(workspace_id, WorkspaceAppStatus::Stopped, None, None);
    }

    fn run_command(&self, workspace: &WorkspaceRecord) -> Result<Option<String>> {
        let repo = self.store.repo(&workspace.repo_id)?;
        let own = self
            .store
            .workspace_by_id(&workspace.id)?
            .and_then(|row| row.run_command);
        Ok(
            crate::harness_config::workspace_run_command(own.as_deref(), &repo)?
                .map(|run| run.command),
        )
    }

    async fn workspace_name(&self, workspace_id: &str) -> String {
        self.workspaces
            .get(workspace_id)
            .await
            .map(|row| row.name)
            .unwrap_or_else(|_| workspace_id.to_string())
    }

    async fn stop_other_instances(&self, workspace_id: &str) -> Result<()> {
        let Ok(workspace) = self.workspaces.get(workspace_id).await else {
            return Ok(());
        };
        let single_instance = self
            .store
            .repo_by_id(&workspace.repo_id)?
            .is_some_and(|repo| repo.single_instance);
        if !single_instance {
            return Ok(());
        }
        let active: Vec<String> = self
            .inner
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, record)| {
                id.as_str() != workspace_id
                    && matches!(
                        record.status,
                        WorkspaceAppStatus::Starting | WorkspaceAppStatus::Running
                    )
            })
            .map(|(id, _)| id.clone())
            .collect();
        for other_id in active {
            let same_repo = self
                .workspaces
                .get(&other_id)
                .await
                .is_ok_and(|other| other.repo_id == workspace.repo_id);
            if same_repo {
                self.stop(&other_id).await?;
            }
        }
        Ok(())
    }

    fn pick_port_with_log(&self, workspace_id: &str, kind: AppKind) -> u16 {
        let used = self.used_ports(workspace_id);
        let mut port = kind.base_port();
        while used.contains(&port) {
            let next = port.saturating_add(1);
            self.process.log(
                workspace_id,
                LogKind::Warning,
                format!("Port {port} is taken by another workspace's app; using {next}"),
            );
            port = next;
        }
        port
    }

    fn next_generation(&self, workspace_id: &str) -> u64 {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .entry(workspace_id.to_string())
            .or_insert(WorkspaceAppRecord {
                status: WorkspaceAppStatus::Stopped,
                port: None,
                https: false,
                exit_code: None,
                generation: 0,
            });
        record.generation += 1;
        let generation = record.generation;
        drop(inner);
        self.runs.send_modify(|runs| *runs += 1);
        generation
    }

    /// Drops a torn-down workspace's app; a monitor still running for it sees it was replaced.
    pub fn forget(&self, workspace_id: &str) {
        self.inner.lock().unwrap().remove(workspace_id);
        self.runs.send_modify(|runs| *runs += 1);
    }

    fn generation(&self, workspace_id: &str) -> u64 {
        self.inner
            .lock()
            .unwrap()
            .get(workspace_id)
            .map(|record| record.generation)
            .unwrap_or(0)
    }

    fn used_ports(&self, except_workspace_id: &str) -> Vec<u16> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, record)| {
                id.as_str() != except_workspace_id
                    && matches!(
                        record.status,
                        WorkspaceAppStatus::Starting | WorkspaceAppStatus::Running
                    )
                    && record.port.is_some()
            })
            .filter_map(|(_, record)| record.port)
            .collect()
    }

    async fn monitor(
        &self,
        workspace: WorkspaceRecord,
        mut events: SessionEvents,
        generation: u64,
    ) {
        let workspace_id = workspace.id.as_str();
        let session = run_session(workspace_id);
        let process = &self.process;
        let startup_deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;
        let mut runs = self.runs.subscribe();

        loop {
            if self.generation(workspace_id) != generation {
                return;
            }
            if let Some(code) = events.exit_code() {
                if process.was_killed(&session) || code == 0 {
                    self.set_status(workspace_id, WorkspaceAppStatus::Stopped, None, None);
                } else {
                    self.set_status(workspace_id, WorkspaceAppStatus::Crashed, None, Some(code));
                    process.log(
                        workspace_id,
                        LogKind::Marker,
                        format!("Crashed (exit {code})"),
                    );
                }
                self.out.changed();
                return;
            }

            // The port only needs finding while the app is starting; once it's running this
            // just waits for it to exit, without rebuilding its whole log on every line.
            let starting = self.runtime(workspace_id).status == WorkspaceAppStatus::Starting;
            if starting {
                let output = process.output(&session);
                if let Some(port) = detect_port(&output) {
                    self.set_status(workspace_id, WorkspaceAppStatus::Running, Some(port), None);
                    self.set_https(workspace_id, serves_https(&output, port));
                    self.out.toast_parts(
                        ToastTone::Ok,
                        vec![
                            ToastPart::Code {
                                value: workspace.name.clone(),
                            },
                            ToastPart::Text {
                                value: " is running on localhost:".into(),
                            },
                            ToastPart::Code {
                                value: port.to_string(),
                            },
                        ],
                        Some(workspace_id.to_string()),
                    );
                    self.out.changed();
                    continue;
                }
            }

            tokio::select! {
                () = events.changed() => {}
                _ = runs.changed() => {}
                () = tokio::time::sleep_until(startup_deadline), if starting => {
                    self.set_status(workspace_id, WorkspaceAppStatus::Crashed, None, None);
                    process.log(workspace_id, LogKind::Marker, "Crashed (startup timed out)");
                    if let Err(error) = process.stop_session(&session) {
                        log::warn!("stopping the app in {workspace_id}: {error}");
                    }
                    self.out.changed();
                    return;
                }
            }
        }
    }

    fn set_status(
        &self,
        workspace_id: &str,
        status: WorkspaceAppStatus,
        port: Option<u16>,
        exit_code: Option<i32>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .entry(workspace_id.to_string())
            .or_insert(WorkspaceAppRecord {
                status: WorkspaceAppStatus::Stopped,
                port: None,
                https: false,
                exit_code: None,
                generation: 0,
            });
        record.status = status;
        if port.is_some() {
            record.port = port;
        }
        if matches!(
            status,
            WorkspaceAppStatus::Stopped | WorkspaceAppStatus::Crashed
        ) {
            record.port = None;
            record.https = false;
        }
        record.exit_code = exit_code;
    }

    fn set_https(&self, workspace_id: &str, https: bool) {
        if let Some(record) = self.inner.lock().unwrap().get_mut(workspace_id) {
            record.https = https;
        }
    }
}

fn run_session(workspace_id: &str) -> SessionId {
    SessionId::of_workspace(workspace_id, "app")
}

pub mod commands {
    use serde::{Deserialize, Serialize};
    use specta::Type;
    use tauri::State;

    use crate::error::Result;
    use crate::state::AppState;

    use super::WorkspaceAppControlAction;

    #[derive(Debug, Clone, Serialize, Deserialize, Type)]
    #[serde(rename_all = "camelCase")]
    pub struct ControlWorkspaceAppInput {
        pub workspace_id: String,
        pub action: WorkspaceAppControlAction,
    }

    /// The workspace's own run command, set in its Output tab; blank runs the repo's.
    #[tauri::command]
    #[specta::specta]
    pub async fn set_workspace_run_command(
        workspace_id: String,
        command: String,
        state: State<'_, AppState>,
    ) -> Result<()> {
        let workspace = state.workspaces.get(&workspace_id).await?;
        let repo = state.store.repo(&workspace.repo_id)?;
        let own = crate::harness_config::own_run_command_to_save(&command, &repo)?;
        state
            .workspaces
            .set_run_command(&workspace_id, own.as_deref())?;
        state.out.changed();
        Ok(())
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn control_workspace_app(
        input: ControlWorkspaceAppInput,
        state: State<'_, AppState>,
    ) -> Result<()> {
        state.apps.control(&input.workspace_id, input.action).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::LogEntry;
    use crate::state::AppState;
    use kind::AppKind;

    #[test]
    fn detects_app_kinds() {
        assert_eq!(AppKind::detect("pnpm storybook"), AppKind::Storybook);
        assert_eq!(AppKind::detect("uvicorn main:app"), AppKind::Python);
        assert_eq!(AppKind::detect("pnpm dev"), AppKind::Vite);
        assert_eq!(AppKind::detect("uv run app"), AppKind::Python);
        // "uv" inside another word isn't uv.
        assert_eq!(AppKind::detect("pnpm dev --mode fluvial"), AppKind::Vite);
        assert_eq!(AppKind::detect("cd ~/dev/uvw && pnpm dev"), AppKind::Vite);
    }

    async fn workspace_with_run_command(
        run_command: &str,
    ) -> (AppState, tempfile::TempDir, WorkspaceRecord) {
        use crate::store::types::RepoRecord;
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("app");
        std::fs::create_dir(&repo).unwrap();
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(dir.path().join("data")));
        state
            .store
            .upsert_repo(&RepoRecord {
                path: repo.to_string_lossy().to_string(),
                name: "app".into(),
                run_command: Some(run_command.into()),
                ..crate::store::fixtures::repo("r1")
            })
            .unwrap();
        crate::settings::Settings::update(
            &state.store,
            crate::settings::SettingsPatch {
                worktree_root: Some(dir.path().join("worktrees").to_string_lossy().into()),
                ..Default::default()
            },
        )
        .unwrap();
        let created = crate::provisioning::create(
            &state,
            crate::provisioning::WorkspaceSpec {
                kind: crate::model::WorkspaceKind::Lead,
                repo_id: "r1".into(),
                name: "Work".into(),
                branch: "feat/x".into(),
                base: "develop".into(),
                pr: None,
                engine: "claude".into(),
                parent_thread_id: None,
                picks: Default::default(),
                setup_message: String::new(),
            },
            |_| Ok(()),
        )
        .await
        .unwrap();
        std::fs::create_dir_all(created.workspace.worktree()).unwrap();
        let workspace = state
            .workspaces
            .set_status(&created.workspace.id, WorkspaceLifecycle::Ready)
            .await
            .unwrap();
        (state, dir, workspace)
    }

    /// Bug: only the webview kept Run from starting while setup ran or had failed; the palette
    /// on a stale snapshot, or any other caller, started the app in a half-made worktree.
    #[tokio::test]
    async fn run_is_refused_with_the_reason_the_snapshot_gives() {
        let (state, _dir, workspace) = workspace_with_run_command("sleep 30").await;
        let id = workspace.id.as_str();
        state
            .workspaces
            .set_status(id, WorkspaceLifecycle::TearingDown)
            .await
            .unwrap();
        for status in [
            WorkspaceLifecycle::ProvisioningFailed,
            WorkspaceLifecycle::Provisioning,
        ] {
            state.workspaces.set_status(id, status).await.unwrap();
            let records = state.workspaces.live().await.unwrap();
            let runs = [WorkspaceRunCommand {
                workspace_id: id.into(),
                command: "sleep 30".into(),
                source: crate::harness_config::RunCommandSource::Repo,
            }];
            let blocks = run_blocks(&records, &runs);

            let refused = state
                .apps
                .control(id, WorkspaceAppControlAction::Run)
                .await
                .unwrap_err();

            assert_eq!(blocks.len(), 1, "{status:?}");
            assert!(refused.to_string().contains(&blocks[0].reason), "{refused}");
            assert_eq!(state.apps.runtime(id).status, WorkspaceAppStatus::Stopped);
        }
        assert_eq!(
            run_block(WorkspaceLifecycle::Idle, false),
            Some("No run command for this workspace.")
        );
        assert_eq!(run_block(WorkspaceLifecycle::Idle, true), None);
    }

    async fn status_becomes(state: &AppState, id: &str, status: WorkspaceAppStatus) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while state.apps.runtime(id).status != status {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{:?}",
                state.apps.runtime(id).status
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    #[tokio::test]
    async fn a_run_finds_its_port_and_a_stop_ends_it() {
        let (state, _dir, workspace) = workspace_with_run_command(
            r#"echo "$HARNESS_BASE on http://localhost:$PORT/"; sleep 30"#,
        )
        .await;
        let id = workspace.id.as_str();

        state
            .apps
            .control(id, WorkspaceAppControlAction::Run)
            .await
            .unwrap();
        status_becomes(&state, id, WorkspaceAppStatus::Running).await;
        assert_eq!(state.apps.runtime(id).port, Some(5173));
        assert!(
            state
                .process
                .log_texts(id)
                .contains(&"develop on http://localhost:5173/".to_string())
        );

        state
            .apps
            .control(id, WorkspaceAppControlAction::Stop)
            .await
            .unwrap();
        assert_eq!(state.apps.runtime(id).status, WorkspaceAppStatus::Stopped);
    }

    /// Bug: teardown forgot every per-workspace cache but the app's, so a torn-down workspace's
    /// app stayed in every snapshot.
    #[tokio::test]
    async fn a_torn_down_workspaces_app_is_forgotten() {
        let (state, _dir, workspace) = workspace_with_run_command("sleep 30").await;
        let id = workspace.id.as_str();
        state
            .apps
            .control(id, WorkspaceAppControlAction::Run)
            .await
            .unwrap();
        status_becomes(&state, id, WorkspaceAppStatus::Starting).await;

        state.process.stop_workspace_sessions(id).await;
        state.forget_workspace(id).await;

        assert!(state.apps.snapshot().is_empty());
    }

    #[tokio::test]
    async fn a_crash_is_reported_after_the_apps_last_lines() {
        let (state, _dir, workspace) =
            workspace_with_run_command("echo one; echo two; exit 3").await;
        let id = workspace.id.as_str();

        state
            .apps
            .control(id, WorkspaceAppControlAction::Run)
            .await
            .unwrap();
        status_becomes(&state, id, WorkspaceAppStatus::Crashed).await;

        assert_eq!(state.apps.runtime(id).exit_code, Some(3));
        let log = state.process.log_snapshot(id);
        assert_eq!(
            log[log.len() - 3..],
            [
                LogEntry::new(LogKind::Output, "one"),
                LogEntry::new(LogKind::Output, "two"),
                LogEntry::new(LogKind::Marker, "Crashed (exit 3)"),
            ]
        );
    }

    /// The log used to print a uvicorn/Storybook-style error that no tool had printed.
    #[test]
    fn a_taken_port_is_noted_in_cormuxs_own_words() {
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        let service = &state.apps;
        service.set_status("a", WorkspaceAppStatus::Running, Some(8000), None);
        assert_eq!(service.pick_port_with_log("b", AppKind::Python), 8001);
        assert_eq!(
            state.process.log_snapshot("b"),
            [LogEntry::new(
                LogKind::Warning,
                "Port 8000 is taken by another workspace's app; using 8001"
            )]
        );
    }
}
