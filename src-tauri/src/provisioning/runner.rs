use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::engines::{EngineKind, SpawnSpec};
use crate::error::{Error, Result};
use crate::ipc::commands::emit_workspace_status;
use crate::ipc::events::StateChanged;
use crate::ipc::types::StateChangeKind;
use crate::process::ProcessSupervisor;
use crate::provisioning::{parse_setup_commands, SETUP_COMMAND_TIMEOUT};
use crate::state::AppState;
use crate::store::types::{ThreadRow, WorkspaceRow};
use crate::workspace::{ThreadActivity, WorkspaceLifecycle, WorkspaceManager, WorkspaceRecord};

#[derive(Debug, Clone)]
pub struct LeadProvisionJob {
    pub workspace_id: String,
    pub thread_id: String,
    pub repo_id: String,
    pub repo_name: String,
    pub setup_commands_raw: String,
    pub engine: String,
    pub goal: String,
    pub review: bool,
}

#[derive(Debug, Clone)]
pub struct JoinProvisionJob {
    pub workspace_id: String,
    pub thread_id: String,
    pub engine: String,
    pub branch: String,
    pub agent_count: usize,
}

pub async fn run_workspace_provisioning(app: AppHandle, job: LeadProvisionJob) {
    let state = app.state::<AppState>();
    if let Err(error) = run_lead(&app, &state, job).await {
        log::warn!("provisioning failed: {error}");
    }
}

pub async fn retry_provisioning(app: AppHandle, workspace_id: String) {
    let state = app.state::<AppState>();
    let snapshot = match state.store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            log::warn!("retry snapshot: {error}");
            return;
        }
    };
    let workspace = match snapshot.workspaces.iter().find(|row| row.id == workspace_id) {
        Some(row) => row.clone(),
        None => return,
    };
    let thread = match snapshot
        .threads
        .iter()
        .find(|row| row.workspace_id == workspace_id && row.title == "Lead")
    {
        Some(row) => row.clone(),
        None => return,
    };
    let repo = match snapshot.repos.iter().find(|row| row.id == workspace.repo_id) {
        Some(row) => row.clone(),
        None => return,
    };
    let goal = snapshot
        .timeline
        .iter()
        .find(|event| event.thread_id == thread.id && event.kind == "message")
        .and_then(|event| serde_json::from_str::<serde_json::Value>(&event.payload).ok())
        .and_then(|value| value.get("text").and_then(|text| text.as_str().map(str::to_string)))
        .unwrap_or_default();

    let _ = state
        .workspace
        .set_status(&workspace_id, WorkspaceLifecycle::Provisioning)
        .await;
    clear_failure(&state, &workspace_id).await;

    let job = LeadProvisionJob {
        workspace_id,
        thread_id: thread.id,
        repo_id: workspace.repo_id,
        repo_name: repo.name,
        setup_commands_raw: repo.setup_commands,
        engine: thread.engine,
        goal,
        review: workspace.kind.as_deref() == Some("review"),
    };
    if let Err(error) = run_lead(&app, &state, job).await {
        log::warn!("retry provisioning failed: {error}");
    }
}

pub async fn skip_provisioning_setup(app: AppHandle, workspace_id: String) {
    let state = app.state::<AppState>();
    let snapshot = match state.store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) => return,
    };
    let workspace = match snapshot.workspaces.iter().find(|row| row.id == workspace_id) {
        Some(row) => row.clone(),
        None => return,
    };
    let thread = match snapshot
        .threads
        .iter()
        .find(|row| row.workspace_id == workspace_id && row.title == "Lead")
    {
        Some(row) => row.clone(),
        None => return,
    };
    let repo = match snapshot.repos.iter().find(|row| row.id == workspace.repo_id) {
        Some(row) => row.clone(),
        None => return,
    };
    let goal = snapshot
        .timeline
        .iter()
        .find(|event| event.thread_id == thread.id && event.kind == "message")
        .and_then(|event| serde_json::from_str::<serde_json::Value>(&event.payload).ok())
        .and_then(|value| value.get("text").and_then(|text| text.as_str().map(str::to_string)))
        .unwrap_or_default();

    let _ = state
        .workspace
        .set_status(&workspace_id, WorkspaceLifecycle::Provisioning)
        .await;
    clear_failure(&state, &workspace_id).await;
    state.process.append_log_line(
        &workspace_id,
        "──── Skipped remaining setup — continuing ────",
    );

    let job = LeadProvisionJob {
        workspace_id: workspace_id.clone(),
        thread_id: thread.id,
        repo_id: workspace.repo_id,
        repo_name: repo.name,
        setup_commands_raw: String::new(),
        engine: thread.engine,
        goal,
        review: workspace.kind.as_deref() == Some("review"),
    };
    if let Err(error) = finish_after_setup(&app, &state, &job, 0).await {
        mark_failed(&app, &state, &workspace_id, None, None, &error.to_string()).await;
    }
}

pub async fn join_thread_provisioning(app: AppHandle, job: JoinProvisionJob) {
    let state = app.state::<AppState>();
    if !workspace_alive(&state, &job.workspace_id).await {
        return;
    }
    set_activity(
        &state,
        &job.workspace_id,
        "Joining worktree…",
        1,
        None,
        None,
    )
    .await;
    persist_workspace_status(&state, &job.workspace_id, "provisioning");
    emit_snapshot(&app, &state);

    let joined = serde_json::json!({
        "icon": "branch",
        "title": "Joined worktree",
        "detail": format!("{} · shared with {} agents", job.branch, job.agent_count),
        "chip": "Worktree ready",
    });
    let _ = state
        .store
        .append_event(&job.thread_id, "tool", &joined.to_string());

    tokio::time::sleep(Duration::from_millis(2400)).await;
    if !workspace_alive(&state, &job.workspace_id).await {
        return;
    }

    set_activity(
        &state,
        &job.workspace_id,
        "Waiting for instructions…",
        0,
        None,
        None,
    )
    .await;

    if let Err(error) = spawn_engine(
        &state,
        &job.thread_id,
        &job.engine,
        &job.workspace_id,
        None,
    )
    .await
    {
        mark_failed(
            &app,
            &state,
            &job.workspace_id,
            None,
            None,
            &error.to_string(),
        )
        .await;
        return;
    }

    let _ = state
        .store
        .upsert_thread(&ThreadRow {
            id: job.thread_id.clone(),
            workspace_id: job.workspace_id.clone(),
            title: snapshot_thread_title(&state, &job.thread_id),
            engine: job.engine,
            session_id: state.engines.session_id(&job.thread_id).ok().flatten(),
            status: "running".into(),
            used_tokens: None,
            context_size: None,
            cost_usd: None,
            transcript_readonly: false,
        });

    let greeting = serde_json::json!({
        "role": "assistant",
        "text": "I'm in the same worktree as the Lead. Tell me which part to take on and I'll coordinate so we don't edit the same files.",
    });
    let _ = state
        .store
        .append_event(&job.thread_id, "message", &greeting.to_string());

    state
        .workspace
        .set_thread(&job.thread_id, &job.workspace_id, ThreadActivity::Running)
        .await;
    let _ = state
        .workspace
        .set_status(&job.workspace_id, WorkspaceLifecycle::Running)
        .await;
    persist_workspace_status(&state, &job.workspace_id, "running");
    emit_snapshot(&app, &state);
}

async fn run_lead(app: &AppHandle, state: &AppState, job: LeadProvisionJob) -> Result<()> {
    if !workspace_alive(state, &job.workspace_id).await {
        return Ok(());
    }

    let record = state
        .workspace
        .get(&job.workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", job.workspace_id)))?;

    let setup = parse_setup_commands(&job.setup_commands_raw);
    let setup_activity = if job.review {
        "Checking out the PR…"
    } else {
        "Running worktree setup…"
    };
    set_activity(
        state,
        &job.workspace_id,
        setup_activity,
        1,
        None,
        None,
    )
    .await;
    emit_snapshot(app, state);

    if !worktree_exists(&record) {
        if let Err(error) = state.workspace.add_worktree(&job.workspace_id).await {
            mark_failed(
                app,
                state,
                &job.workspace_id,
                None,
                None,
                &error.to_string(),
            )
            .await;
            return Ok(());
        }
        if !workspace_alive(state, &job.workspace_id).await {
            return Ok(());
        }
    }

    if !setup.is_empty() {
        if !run_setup_commands(app, state, &job, &record, &setup).await? {
            return Ok(());
        }
    } else {
        state.process.append_log_line(
            &job.workspace_id,
            "No setup commands configured for this repo",
        );
    }

    finish_after_setup(app, state, &job, setup.len()).await
}

async fn run_setup_commands(
    app: &AppHandle,
    state: &AppState,
    job: &LeadProvisionJob,
    record: &WorkspaceRecord,
    setup: &[String],
) -> Result<bool> {
    let log_id = &job.workspace_id;
    state.process.append_log_line(
        log_id,
        format!(
            "Running {} setup command{} from {} settings",
            setup.len(),
            if setup.len() == 1 { "" } else { "s" },
            job.repo_name,
        ),
    );

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let worktree = PathBuf::from(&record.worktree_path);
    let harness_env = WorkspaceManager::harness_env(record);

    for (index, command) in setup.iter().enumerate() {
        if !workspace_alive(state, &job.workspace_id).await {
            return Ok(false);
        }

        let started = Instant::now();
        state
            .process
            .append_log_line(log_id, format!("cmd: {command}"));
        state.process.append_log_line(
            log_id,
            format!("── Command {} of {} ──", index + 1, setup.len()),
        );

        let session_id = format!("{}-setup-{index}", job.workspace_id);
        state.process.spawn_session(
            &session_id,
            &shell,
            &["-l", "-c", command],
            Some(&worktree),
            &harness_env,
        )?;

        let code = match poll_session(
            &state.process,
            &session_id,
            log_id,
            SETUP_COMMAND_TIMEOUT,
        )
        .await
        {
            Ok(code) => code,
            Err(error) => {
                mark_failed(
                    app,
                    state,
                    &job.workspace_id,
                    Some(command.clone()),
                    Some(-1),
                    &error.to_string(),
                )
                .await;
                return Ok(false);
            }
        };

        forward_session_lines(&state.process, &session_id, log_id);
        let _ = state.process.stop_session(&session_id);

        state.process.append_log_line(
            log_id,
            format!(
                "Finished command {} of {} in {:.1}s",
                index + 1,
                setup.len(),
                started.elapsed().as_secs_f32(),
            ),
        );

        if code != 0 {
            state.process.append_log_line(
                log_id,
                format!("Setup failed: exit code {code}"),
            );
            mark_failed(
                app,
                state,
                &job.workspace_id,
                Some(command.clone()),
                Some(code),
                &format!("Setup command failed with exit code {code}"),
            )
            .await;
            return Ok(false);
        }
    }

    state
        .process
        .append_log_line(log_id, "──── Worktree ready ────");
    Ok(true)
}

async fn finish_after_setup(
    app: &AppHandle,
    state: &AppState,
    job: &LeadProvisionJob,
    setup_count: usize,
) -> Result<()> {
    if !workspace_alive(state, &job.workspace_id).await {
        return Ok(());
    }

    set_activity(state, &job.workspace_id, "Starting agent…", 2, None, None).await;
    emit_snapshot(app, state);

    spawn_engine(
        state,
        &job.thread_id,
        &job.engine,
        &job.workspace_id,
        Some(&job.goal),
    )
    .await?;

    let setup_step = if setup_count > 0 {
        serde_json::json!({
            "icon": "terminal",
            "title": "Ran worktree setup",
            "detail": format!("{setup_count} commands from {} settings, see the Output tab", job.repo_name),
            "chip": "Worktree ready",
        })
    } else {
        serde_json::json!({
            "icon": "branch",
            "title": "Worktree ready",
            "detail": format!("No setup commands configured for {}", job.repo_name),
            "chip": "Worktree ready",
        })
    };
    let _ = state
        .store
        .append_event(&job.thread_id, "tool", &setup_step.to_string());

    seed_summary_if_needed(state, job).await?;

    let _ = state.store.upsert_thread(&ThreadRow {
        id: job.thread_id.clone(),
        workspace_id: job.workspace_id.clone(),
        title: "Lead".into(),
        engine: job.engine.clone(),
        session_id: state.engines.session_id(&job.thread_id).ok().flatten(),
        status: "running".into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
    });

    state
        .workspace
        .set_thread(&job.thread_id, &job.workspace_id, ThreadActivity::Running)
        .await;
    let record = state
        .workspace
        .set_status(&job.workspace_id, WorkspaceLifecycle::Running)
        .await?;
    set_activity(
        state,
        &job.workspace_id,
        initial_running_activity(&job.engine, job.review),
        0,
        None,
        None,
    )
    .await;

    persist_workspace_row(state, &job.workspace_id, "running", &record)?;
    emit_workspace_status(app, state, &job.workspace_id, WorkspaceLifecycle::Running);
    emit_snapshot(app, state);
    Ok(())
}

fn worktree_exists(record: &WorkspaceRecord) -> bool {
    Path::new(&record.worktree_path).exists()
}

async fn spawn_engine(
    state: &AppState,
    thread_id: &str,
    engine: &str,
    workspace_id: &str,
    goal: Option<&str>,
) -> Result<()> {
    let workspace = state
        .workspace
        .get(workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let kind = parse_engine_kind(engine);
    state
        .engines
        .spawn(SpawnSpec {
            thread_id: thread_id.to_string(),
            kind,
            cwd: PathBuf::from(&workspace.worktree_path),
            resume: None,
            override_argv: None,
            auto_approve_readonly: true,
        })
        .await?;
    if let Some(text) = goal.filter(|value| !value.is_empty()) {
        state.engines.prompt(thread_id, text)?;
    }
    Ok(())
}

fn parse_engine_kind(engine: &str) -> EngineKind {
    match engine.to_lowercase().as_str() {
        "cursor" => EngineKind::Cursor,
        "codex" => EngineKind::Codex,
        "gemini" => EngineKind::Gemini,
        _ => EngineKind::Claude,
    }
}

fn initial_running_activity(engine: &str, review: bool) -> &'static str {
    if review {
        return "Reading the diff…";
    }
    if engine.eq_ignore_ascii_case("claude") {
        "Reading repo…"
    } else {
        "Drafting plan…"
    }
}

async fn seed_summary_if_needed(state: &AppState, job: &LeadProvisionJob) -> Result<()> {
    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == job.workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", job.workspace_id)))?;
    if workspace.summary.as_ref().is_some_and(|text| !text.is_empty()) {
        return Ok(());
    }

    let record = state
        .workspace
        .get(&job.workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", job.workspace_id)))?;
    let engine_label = engine_display(&job.engine);
    let summary = if job.review {
        format!(
            "Reviewing changes on {}. {engine_label} is reading the diff and will draft comments for your approval; nothing has been posted to GitHub yet.",
            record.branch
        )
    } else if job.engine.eq_ignore_ascii_case("claude") {
        format!(
            "Just started on “{}”, branched from {}. {engine_label} is reading the repo to find where the change belongs; no files have been touched yet.",
            workspace.name, record.base
        )
    } else {
        format!(
            "Just started on “{}”, branched from {}. {engine_label} is drafting a step-by-step plan for your approval; no files have been touched yet.",
            workspace.name, record.base
        )
    };
    let now = chrono_timestamp();
    let _ = state.store.upsert_workspace(&WorkspaceRow {
        id: workspace.id,
        repo_id: workspace.repo_id,
        name: workspace.name,
        branch: workspace.branch,
        worktree_path: workspace.worktree_path,
        status: workspace.status,
        created_at: workspace.created_at,
        summary: Some(summary),
        summary_at: Some(now.clone()),
        summary_source: workspace.summary_source,
        kind: workspace.kind,
        pr_number: workspace.pr_number,
        modified_files: workspace.modified_files,
        archived_at: workspace.archived_at,
    });
    Ok(())
}

fn engine_display(engine: &str) -> String {
    match engine.to_lowercase().as_str() {
        "cursor" => "Cursor".into(),
        "codex" => "Codex".into(),
        "gemini" => "Gemini".into(),
        _ => "Claude Code".into(),
    }
}

async fn mark_failed(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    failed_command: Option<String>,
    exit_code: Option<i32>,
    message: &str,
) {
    set_activity(
        state,
        workspace_id,
        "Setup failed",
        0,
        failed_command,
        exit_code,
    )
    .await;
    let _ = state
        .workspace
        .set_status(workspace_id, WorkspaceLifecycle::ProvisioningFailed)
        .await;
    persist_workspace_status(state, workspace_id, "provisioningFailed");
    emit_workspace_status(
        app,
        state,
        workspace_id,
        WorkspaceLifecycle::ProvisioningFailed,
    );
    emit_snapshot(app, state);
    crate::feedback::emit_toast(
        app,
        crate::ipc::types::ToastRaisedPayload {
            tone: crate::ipc::types::ToastTone::Bad,
            parts: vec![crate::ipc::types::ToastPart::Text {
                value: message.to_string(),
            }],
            workspace_id: Some(workspace_id.to_string()),
        },
    );
}

async fn clear_failure(state: &AppState, workspace_id: &str) {
    set_activity(state, workspace_id, "Running worktree setup…", 1, None, None).await;
}

async fn set_activity(
    state: &AppState,
    workspace_id: &str,
    activity: &str,
    prov_step: u8,
    failed_command: Option<String>,
    exit_code: Option<i32>,
) {
    let _ = state
        .workspace
        .set_provisioning_detail(
            workspace_id,
            activity,
            prov_step,
            failed_command,
            exit_code,
        )
        .await;
}

async fn workspace_alive(state: &AppState, workspace_id: &str) -> bool {
    match state.workspace.get(workspace_id).await {
        Some(record) => !matches!(
            record.status,
            WorkspaceLifecycle::TearingDown | WorkspaceLifecycle::Gone
        ),
        None => false,
    }
}

fn forward_session_lines(process: &ProcessSupervisor, from: &str, to: &str) {
    for line in process.drain_pending(from) {
        process.append_log_line(to, line);
    }
}

async fn poll_session(
    process: &ProcessSupervisor,
    session_id: &str,
    log_id: &str,
    timeout: Duration,
) -> Result<i32> {
    let started = Instant::now();
    loop {
        forward_session_lines(process, session_id, log_id);
        if let Some(code) = process.exit_code(session_id) {
            return Ok(code);
        }
        if started.elapsed() > timeout {
            return Err(Error::Process("timed out waiting for setup command".into()));
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}

async fn wait_session(
    process: &ProcessSupervisor,
    session_id: &str,
    timeout: Duration,
) -> Result<i32> {
    poll_session(process, session_id, session_id, timeout).await
}

fn persist_workspace_status(state: &AppState, workspace_id: &str, status: &str) {
    let Ok(snapshot) = state.store.snapshot() else {
        return;
    };
    let Some(row) = snapshot.workspaces.iter().find(|row| row.id == workspace_id) else {
        return;
    };
    let _ = state.store.upsert_workspace(&WorkspaceRow {
        status: status.into(),
        ..row.clone()
    });
}

fn persist_workspace_row(
    state: &AppState,
    workspace_id: &str,
    status: &str,
    record: &WorkspaceRecord,
) -> Result<()> {
    let snapshot = state.store.snapshot()?;
    let Some(row) = snapshot.workspaces.iter().find(|row| row.id == workspace_id) else {
        return Ok(());
    };
    state.store.upsert_workspace(&WorkspaceRow {
        id: row.id.clone(),
        repo_id: row.repo_id.clone(),
        name: row.name.clone(),
        branch: record.branch.clone(),
        worktree_path: record.worktree_path.clone(),
        status: status.into(),
        created_at: row.created_at.clone(),
        summary: row.summary.clone(),
        summary_at: row.summary_at.clone(),
        summary_source: row.summary_source.clone(),
        kind: row.kind.clone(),
        pr_number: row.pr_number,
        modified_files: row.modified_files,
        archived_at: row.archived_at.clone(),
    })
}

fn emit_snapshot(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
}

fn snapshot_thread_title(state: &AppState, thread_id: &str) -> String {
    state
        .store
        .snapshot()
        .ok()
        .and_then(|snapshot| {
            snapshot
                .threads
                .iter()
                .find(|row| row.id == thread_id)
                .map(|row| row.title.clone())
        })
        .unwrap_or_else(|| "Agent".into())
}

fn chrono_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessSupervisor;
    use crate::shell_env::ShellEnv;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn setup_command_mock_true_then_false() {
        let process = ProcessSupervisor::new(Arc::new(RwLock::new(ShellEnv::new())));
        let log_id = "test-ws";
        let shell = "/bin/sh";
        process
            .spawn_session(log_id, shell, &["-c", "echo ok"], None, &[])
            .unwrap();
        let code = wait_session(&process, log_id, Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(code, 0);
        forward_session_lines(&process, log_id, log_id);
        assert!(process.drain_pending(log_id).iter().any(|l| l.contains("ok")));

        let fail_id = "test-ws-fail";
        process
            .spawn_session(fail_id, shell, &["-c", "exit 7"], None, &[])
            .unwrap();
        let code = wait_session(&process, fail_id, Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(code, 7);
    }
}
