mod kind;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kind::{AppKind, port_in_use_message};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::error::{Error, Result};
use crate::feedback::emit_toast_parts;
use crate::ipc::events::StateChanged;
use crate::ipc::types::{StateChangeKind, ToastPart, ToastTone};
use crate::process::{ProcessSupervisor, detect_port};
use crate::state::AppState;
use crate::store::Store;
use crate::workspace::WorkspaceManager;

const RUN_SESSION_SUFFIX: &str = "-app";

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
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceAppAction {
    Run,
    Restart,
    Stop,
    Clear,
}

struct WorkspaceAppRecord {
    status: WorkspaceAppStatus,
    port: Option<u16>,
    exit_code: Option<i32>,
    kind: AppKind,
    quiet_stop: bool,
    /// Bumped on every run so a monitor from an earlier run knows to exit.
    generation: u64,
}

#[derive(Clone)]
pub struct WorkspaceAppService {
    inner: Arc<Mutex<HashMap<String, WorkspaceAppRecord>>>,
}

impl WorkspaceAppService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
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
                exit_code: record.exit_code,
            })
            .unwrap_or(WorkspaceAppRuntime {
                workspace_id: workspace_id.to_string(),
                status: WorkspaceAppStatus::Stopped,
                port: None,
                exit_code: None,
            })
    }

    pub async fn control(
        &self,
        app: &AppHandle,
        state: &AppState,
        workspace_id: &str,
        action: WorkspaceAppAction,
    ) -> Result<()> {
        match action {
            WorkspaceAppAction::Clear => {
                state.process.clear_log(workspace_id);
                self.emit_changed(app, state);
                Ok(())
            }
            WorkspaceAppAction::Stop => {
                self.stop(app, state, workspace_id, false).await?;
                Ok(())
            }
            WorkspaceAppAction::Restart => {
                self.restart(app, state, workspace_id).await?;
                Ok(())
            }
            WorkspaceAppAction::Run => {
                self.run(app, state, workspace_id).await?;
                Ok(())
            }
        }
    }

    pub fn restart_approved(
        &self,
        process: &ProcessSupervisor,
        store: &Store,
        workspace_id: &str,
    ) -> Result<()> {
        let workspace = store
            .workspace_by_id(workspace_id)?
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        let repo = store
            .snapshot()?
            .repos
            .into_iter()
            .find(|repo| repo.id == workspace.repo_id)
            .ok_or_else(|| Error::Workspace(format!("unknown repo {}", workspace.repo_id)))?;
        let repo_path = crate::ipc::commands::expand_tilde(&repo.path);
        let run_command =
            crate::harness_config::effective_run_command(&repo.run_command, &repo_path)
                .ok_or_else(|| Error::Process("no run command configured".into()))?;
        let repo_for_env = repo.clone();

        let session_id = run_session_id(workspace_id);
        let _ = process.stop_session(&session_id);
        process.append_log_line(workspace_id, "^C");
        process.append_log_line(workspace_id, "──── Restarting ────");

        self.set_status(workspace_id, WorkspaceAppStatus::Starting, None, None);
        let kind = AppKind::detect(&run_command);
        let port = self.pick_port_with_log(process, workspace_id, kind);
        let mut env = workspace_from_row(&workspace, Some(&repo_for_env));
        env.push(("PORT".into(), port.to_string()));
        spawn_run(
            process,
            workspace_id,
            &workspace.worktree_path,
            &run_command,
            &env,
        )?;
        Ok(())
    }

    async fn run(&self, app: &AppHandle, state: &AppState, workspace_id: &str) -> Result<()> {
        let (worktree, run_command, kind, name) = run_context(state, workspace_id).await?;
        let port = self.pick_port_with_log(&state.process, workspace_id, kind);
        let env = run_env(state, workspace_id, port).await?;

        let generation = self.set_record(workspace_id, kind);
        self.set_status(workspace_id, WorkspaceAppStatus::Starting, Some(port), None);
        state
            .process
            .append_log_line(workspace_id, format!("> {run_command}"));

        spawn_run(&state.process, workspace_id, &worktree, &run_command, &env)?;

        let app_handle = app.clone();
        let workspace_id = workspace_id.to_string();
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            service
                .monitor(app_handle, workspace_id, name, generation)
                .await;
        });

        self.emit_changed(app, state);
        Ok(())
    }

    async fn restart(&self, app: &AppHandle, state: &AppState, workspace_id: &str) -> Result<()> {
        let name = workspace_name(state, workspace_id).await?;
        emit_toast_parts(
            app,
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
        self.stop(app, state, workspace_id, true).await?;
        state
            .process
            .append_log_line(workspace_id, "──── Restarting ────");
        self.run(app, state, workspace_id).await?;
        Ok(())
    }

    async fn stop(
        &self,
        app: &AppHandle,
        state: &AppState,
        workspace_id: &str,
        quiet: bool,
    ) -> Result<()> {
        let session_id = run_session_id(workspace_id);
        let _ = state.process.stop_session(&session_id);
        state.process.append_log_line(workspace_id, "^C");
        if !quiet {
            state
                .process
                .append_log_line(workspace_id, "──── App stopped ────");
            let name = workspace_name(state, workspace_id).await?;
            emit_toast_parts(
                app,
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
        self.emit_changed(app, state);
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
            process.append_log_line(workspace_id, port_in_use_message(kind, port));
            port = port.saturating_add(1);
        }
        port
    }

    fn set_record(&self, workspace_id: &str, kind: AppKind) -> u64 {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .entry(workspace_id.to_string())
            .or_insert(WorkspaceAppRecord {
                status: WorkspaceAppStatus::Stopped,
                port: None,
                exit_code: None,
                kind,
                quiet_stop: false,
                generation: 0,
            });
        record.kind = kind;
        record.generation += 1;
        record.generation
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
        app: AppHandle,
        workspace_id: String,
        workspace_name: String,
        generation: u64,
    ) {
        let state = app.state::<AppState>();
        let session_id = run_session_id(&workspace_id);
        let log_id = workspace_id.as_str();
        let started = std::time::Instant::now();
        let mut running_toast_sent = false;

        loop {
            let process = state.process.clone();
            if self.generation(&workspace_id) != generation {
                // A restart replaced this run; its own monitor takes over.
                break;
            }
            for line in process.drain_pending(&session_id) {
                process.append_log_line(log_id, line);
            }
            if let Some(code) = process.exit_code(&session_id) {
                // The reader can land the last lines just after exit.
                tokio::time::sleep(Duration::from_millis(50)).await;
                for line in process.drain_pending(&session_id) {
                    process.append_log_line(log_id, line);
                }
                let killed = process.was_killed(&session_id);
                if killed {
                    self.set_status(&workspace_id, WorkspaceAppStatus::Stopped, None, None);
                } else if code != 0 {
                    self.set_status(&workspace_id, WorkspaceAppStatus::Crashed, None, Some(code));
                    process.append_log_line(log_id, format!("──── Crashed (exit {code}) ────"));
                } else {
                    self.set_status(&workspace_id, WorkspaceAppStatus::Stopped, None, None);
                }
                let version = state.bump_event_version();
                let _ = StateChanged {
                    version,
                    kind: StateChangeKind::WorkspaceApp,
                }
                .emit(&app);
                break;
            }

            if let Some(detected) = detect_port(&process.output_session(&session_id))
                && self.runtime(&workspace_id).status == WorkspaceAppStatus::Starting
            {
                self.set_status(
                    &workspace_id,
                    WorkspaceAppStatus::Running,
                    Some(detected),
                    None,
                );
                if !running_toast_sent {
                    running_toast_sent = true;
                    emit_toast_parts(
                        &app,
                        ToastTone::Ok,
                        vec![
                            ToastPart::Code {
                                value: workspace_name.clone(),
                            },
                            ToastPart::Text {
                                value: " is running on localhost:".into(),
                            },
                            ToastPart::Code {
                                value: detected.to_string(),
                            },
                        ],
                        Some(workspace_id.clone()),
                    );
                }
                let version = state.bump_event_version();
                let _ = StateChanged {
                    version,
                    kind: StateChangeKind::WorkspaceApp,
                }
                .emit(&app);
            }

            if started.elapsed() > Duration::from_secs(120)
                && self.runtime(&workspace_id).status == WorkspaceAppStatus::Starting
            {
                self.set_status(&workspace_id, WorkspaceAppStatus::Crashed, None, Some(-1));
                process.append_log_line(log_id, "──── Crashed (startup timed out) ────");
                let _ = process.stop_session(&session_id);
                let version = state.bump_event_version();
                let _ = StateChanged {
                    version,
                    kind: StateChangeKind::WorkspaceApp,
                }
                .emit(&app);
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
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
                exit_code: None,
                kind: AppKind::Vite,
                quiet_stop: false,
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
        }
        record.exit_code = exit_code;
    }

    fn emit_changed(&self, app: &AppHandle, state: &AppState) {
        let version = state.bump_event_version();
        let _ = StateChanged {
            version,
            kind: StateChangeKind::WorkspaceApp,
        }
        .emit(app);
    }
}

impl Default for WorkspaceAppService {
    fn default() -> Self {
        Self::new()
    }
}

fn run_session_id(workspace_id: &str) -> String {
    format!("{workspace_id}{RUN_SESSION_SUFFIX}")
}

fn spawn_run(
    process: &ProcessSupervisor,
    workspace_id: &str,
    worktree_path: &str,
    run_command: &str,
    extra_env: &[(String, String)],
) -> Result<()> {
    let session_id = run_session_id(workspace_id);
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    process.spawn_session(
        &session_id,
        &shell,
        &["-l", "-c", run_command],
        Some(Path::new(worktree_path)),
        extra_env,
    )?;
    Ok(())
}

fn workspace_from_row(
    row: &crate::store::types::WorkspaceRow,
    repo: Option<&crate::store::types::RepoRecord>,
) -> Vec<(String, String)> {
    let record = crate::workspace::WorkspaceRecord {
        id: row.id.clone(),
        repo_id: row.repo_id.clone(),
        repo_path: repo.map(|r| r.path.clone()).unwrap_or_default(),
        name: row.name.clone(),
        branch: row.branch.clone(),
        base: "main".into(),
        worktree_path: row.worktree_path.clone(),
        status: crate::workspace::WorkspaceLifecycle::Ready,
        version: 0,
        activity: String::new(),
        prov_step: 0,
        setup_failed_command: None,
        setup_failed_exit_code: None,
    };
    WorkspaceManager::harness_env(&record)
}

async fn run_context(
    state: &AppState,
    workspace_id: &str,
) -> Result<(String, String, AppKind, String)> {
    let workspace = state
        .workspace
        .get(workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let repo = state
        .store
        .snapshot()?
        .repos
        .into_iter()
        .find(|repo| repo.id == workspace.repo_id)
        .ok_or_else(|| Error::Workspace(format!("unknown repo {}", workspace.repo_id)))?;
    let repo_path = crate::ipc::commands::expand_tilde(&repo.path);
    let run_command = crate::harness_config::effective_run_command(&repo.run_command, &repo_path)
        .ok_or_else(|| Error::Process("no run command configured".into()))?;
    let kind = AppKind::detect(&run_command);
    Ok((workspace.worktree_path, run_command, kind, workspace.name))
}

async fn run_env(state: &AppState, workspace_id: &str, port: u16) -> Result<Vec<(String, String)>> {
    let workspace = state
        .workspace
        .get(workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let mut env = WorkspaceManager::harness_env(&workspace);
    env.push(("PORT".into(), port.to_string()));
    Ok(env)
}

async fn workspace_name(state: &AppState, workspace_id: &str) -> Result<String> {
    Ok(state
        .workspace
        .get(workspace_id)
        .await
        .map(|row| row.name)
        .unwrap_or_else(|| workspace_id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kind::AppKind;

    #[test]
    fn detects_app_kinds() {
        assert_eq!(AppKind::detect("pnpm storybook"), AppKind::Storybook);
        assert_eq!(AppKind::detect("uvicorn main:app"), AppKind::Python);
        assert_eq!(AppKind::detect("pnpm dev"), AppKind::Vite);
    }

    #[test]
    fn reserves_ports_per_workspace() {
        let service = WorkspaceAppService::new();
        service.set_status("a", WorkspaceAppStatus::Running, Some(5173), None);
        service.set_status("b", WorkspaceAppStatus::Starting, None, None);
        let used = service.used_ports("b");
        assert!(used.contains(&5173));
    }
}
