mod kind;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kind::AppKind;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::watch;

use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastTone};
use crate::process::{
    LogKind, ProcessSupervisor, SessionEvents, SessionId, ShellCommand, detect_port, serves_https,
};
use crate::state::AppState;
use crate::workspaces::WorkspaceRecord;

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
    inner: Arc<Mutex<HashMap<String, WorkspaceAppRecord>>>,
    /// Bumped with any workspace's generation, so monitors notice they've been replaced.
    runs: Arc<watch::Sender<u64>>,
}

impl WorkspaceAppService {
    pub fn new() -> Self {
        Self {
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
        state: &AppState,
        workspace_id: &str,
        action: WorkspaceAppControlAction,
    ) -> Result<()> {
        match action {
            WorkspaceAppControlAction::Clear => {
                state.process.clear_log(workspace_id);
                state.out.changed();
                Ok(())
            }
            WorkspaceAppControlAction::Stop => {
                self.stop(state, workspace_id, false).await?;
                Ok(())
            }
            WorkspaceAppControlAction::Restart => {
                self.restart(state, workspace_id).await?;
                Ok(())
            }
            WorkspaceAppControlAction::Run => {
                self.run(state, workspace_id).await?;
                Ok(())
            }
        }
    }

    async fn run(&self, state: &AppState, workspace_id: &str) -> Result<()> {
        let workspace = state.workspaces.get(workspace_id).await?;
        let run_command = run_command(state, &workspace)?;
        let kind = AppKind::detect(&run_command);
        self.stop_other_instances(state, workspace_id).await?;
        let port = self.pick_port_with_log(&state.process, workspace_id, kind);
        let mut env = workspace.harness_env().vars();
        env.push(("PORT".into(), port.to_string()));

        let generation = self.next_generation(workspace_id);
        self.set_status(workspace_id, WorkspaceAppStatus::Starting, Some(port), None);
        state
            .process
            .log(workspace_id, LogKind::Command, run_command.as_str());

        let command = ShellCommand {
            cmd: run_command,
            cwd: workspace.worktree(),
            env,
        };
        let events = state
            .process
            .spawn_shell(run_session(workspace_id), &command, Some(workspace_id))
            .await?;

        let state_bg = state.clone();
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            service
                .monitor(state_bg, workspace, events, generation)
                .await;
        });

        state.out.changed();
        Ok(())
    }

    async fn restart(&self, state: &AppState, workspace_id: &str) -> Result<()> {
        let name = workspace_name(state, workspace_id).await?;
        state.out.toast_parts(
            ToastTone::Default,
            vec![
                ToastPart::Text {
                    value: "Restarting the app in ".into(),
                },
                ToastPart::Code {
                    value: name.clone(),
                },
            ],
            Some(workspace_id.to_string()),
        );
        self.stop(state, workspace_id, true).await?;
        state
            .process
            .log(workspace_id, LogKind::Marker, "Restarting");
        self.run(state, workspace_id).await?;
        Ok(())
    }

    async fn stop(&self, state: &AppState, workspace_id: &str, quiet: bool) -> Result<()> {
        if let Err(error) = state.process.stop_session(&run_session(workspace_id)) {
            log::warn!("stopping the app in {workspace_id}: {error}");
        }
        state.process.log(workspace_id, LogKind::Interrupt, "^C");
        if !quiet {
            state
                .process
                .log(workspace_id, LogKind::Marker, "App stopped");
            let name = workspace_name(state, workspace_id).await?;
            state.out.toast_parts(
                ToastTone::Default,
                vec![
                    ToastPart::Text {
                        value: "Stopped the app in ".into(),
                    },
                    ToastPart::Code { value: name },
                ],
                Some(workspace_id.to_string()),
            );
        }
        self.set_status(workspace_id, WorkspaceAppStatus::Stopped, None, None);
        state.out.changed();
        Ok(())
    }

    /// With the repo's "one instance at a time" on, stops its app in every other workspace.
    async fn stop_other_instances(&self, state: &AppState, workspace_id: &str) -> Result<()> {
        let Ok(workspace) = state.workspaces.get(workspace_id).await else {
            return Ok(());
        };
        let single_instance = state
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
            let same_repo = state
                .workspaces
                .get(&other_id)
                .await
                .is_ok_and(|other| other.repo_id == workspace.repo_id);
            if same_repo {
                self.stop(state, &other_id, false).await?;
            }
        }
        Ok(())
    }

    fn pick_port_with_log(
        &self,
        process: &ProcessSupervisor,
        workspace_id: &str,
        kind: AppKind,
    ) -> u16 {
        let used = self.used_ports(workspace_id);
        let mut port = kind.base_port();
        while used.contains(&port) {
            let next = port.saturating_add(1);
            process.log(
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

    /// Follows one run until it exits or another run replaces it: finds its port while it
    /// starts, and reports how it ended.
    async fn monitor(
        &self,
        state: AppState,
        workspace: WorkspaceRecord,
        mut events: SessionEvents,
        generation: u64,
    ) {
        let workspace_id = workspace.id.as_str();
        let session = run_session(workspace_id);
        let process = &state.process;
        let startup_deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;
        let mut runs = self.runs.subscribe();

        loop {
            if self.generation(workspace_id) != generation {
                // Another run replaced this one; its own monitor takes over.
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
                state.out.changed();
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
                    state.out.toast_parts(
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
                    state.out.changed();
                    continue;
                }
            }

            tokio::select! {
                () = events.changed() => {}
                _ = runs.changed() => {}
                () = tokio::time::sleep_until(startup_deadline), if starting => {
                    self.set_status(workspace_id, WorkspaceAppStatus::Crashed, None, Some(-1));
                    process.log(workspace_id, LogKind::Marker, "Crashed (startup timed out)");
                    if let Err(error) = process.stop_session(&session) {
                        log::warn!("stopping the app in {workspace_id}: {error}");
                    }
                    state.out.changed();
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

impl Default for WorkspaceAppService {
    fn default() -> Self {
        Self::new()
    }
}

fn run_session(workspace_id: &str) -> SessionId {
    SessionId::Run {
        workspace: workspace_id.to_string(),
    }
}

fn run_command(state: &AppState, workspace: &WorkspaceRecord) -> Result<String> {
    let repo = state.store.repo(&workspace.repo_id)?;
    let own = state
        .store
        .workspace_by_id(&workspace.id)?
        .and_then(|row| row.run_command);
    crate::harness_config::workspace_run_command(own.as_deref(), &repo)?
        .map(|run| run.command)
        .ok_or_else(|| Error::Process("no run command configured".into()))
}

async fn workspace_name(state: &AppState, workspace_id: &str) -> Result<String> {
    Ok(state
        .workspaces
        .get(workspace_id)
        .await
        .map(|row| row.name)
        .unwrap_or_else(|_| workspace_id.to_string()))
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
        state
            .apps
            .control(&state, &input.workspace_id, input.action)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::LogEntry;
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
            },
        )
        .await
        .unwrap();
        std::fs::create_dir_all(created.workspace.worktree()).unwrap();
        (state, dir, created.workspace)
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
            .control(&state, id, WorkspaceAppControlAction::Run)
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
            .control(&state, id, WorkspaceAppControlAction::Stop)
            .await
            .unwrap();
        assert_eq!(state.apps.runtime(id).status, WorkspaceAppStatus::Stopped);
    }

    #[tokio::test]
    async fn a_crash_is_reported_after_the_apps_last_lines() {
        let (state, _dir, workspace) =
            workspace_with_run_command("echo one; echo two; exit 3").await;
        let id = workspace.id.as_str();

        state
            .apps
            .control(&state, id, WorkspaceAppControlAction::Run)
            .await
            .unwrap();
        status_becomes(&state, id, WorkspaceAppStatus::Crashed).await;

        assert_eq!(state.apps.runtime(id).exit_code, Some(3));
        let log = state.process.take_log_snapshot(id);
        assert_eq!(
            log[log.len() - 3..],
            [
                LogEntry::new(LogKind::Output, "one"),
                LogEntry::new(LogKind::Output, "two"),
                LogEntry::new(LogKind::Marker, "Crashed (exit 3)"),
            ]
        );
    }

    #[test]
    fn reserves_ports_per_workspace() {
        let service = WorkspaceAppService::new();
        service.set_status("a", WorkspaceAppStatus::Running, Some(5173), None);
        service.set_status("b", WorkspaceAppStatus::Starting, None, None);
        let used = service.used_ports("b");
        assert!(used.contains(&5173));
    }

    /// The log used to print a uvicorn/Storybook-style error that no tool had printed.
    #[test]
    fn a_taken_port_is_noted_in_cormuxs_own_words() {
        let service = WorkspaceAppService::new();
        service.set_status("a", WorkspaceAppStatus::Running, Some(8000), None);
        let process = ProcessSupervisor::new(Default::default());
        assert_eq!(
            service.pick_port_with_log(&process, "b", AppKind::Python),
            8001
        );
        assert_eq!(
            process.take_log_snapshot("b"),
            [LogEntry::new(
                LogKind::Warning,
                "Port 8000 is taken by another workspace's app; using 8001"
            )]
        );
    }
}
