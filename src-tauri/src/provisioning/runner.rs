use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::engines::prompt::Prompt;
use crate::engines::{EngineKind, SpawnSpec};
use crate::error::{Error, Result};
use crate::feedback::Output;
use crate::ipc::types::StateChangeKind;
use crate::process::ProcessSupervisor;
use crate::provisioning::{SETUP_COMMAND_TIMEOUT, parse_setup_commands};
use crate::state::AppState;
use crate::store::types::{ThreadRow, WorkspaceRow};
use crate::timeline::{TimelineStep, record_step};
use crate::workspace::{ThreadStatus, WorkspaceLifecycle, WorkspaceManager, WorkspaceRecord};

#[derive(Debug, Clone)]
pub struct LeadProvisionJob {
    pub workspace_id: String,
    pub thread_id: String,
    pub repo_name: String,
    pub setup_commands_raw: String,
    pub engine: String,
    pub goal: String,
    /// What the engine is sent first, when it isn't just the goal: the goal with its
    /// attachments. Lost on a retry, which sends the goal as the thread shows it.
    pub prompt: Option<Prompt>,
    pub review: bool,
}

pub async fn run_workspace_provisioning(out: Output, state: AppState, job: LeadProvisionJob) {
    if let Err(error) = run_lead(&out, &state, job).await {
        log::warn!("provisioning failed: {error}");
    }
}

pub async fn retry_provisioning(out: Output, state: AppState, workspace_id: String) {
    let Some((workspace, thread, repo)) = provisioning_rows(&state, &workspace_id) else {
        return;
    };
    let goal = state
        .store
        .first_message_text(&thread.id)
        .ok()
        .flatten()
        .unwrap_or_default();

    let _ = state
        .workspace
        .set_status(&workspace_id, WorkspaceLifecycle::Provisioning)
        .await;
    clear_failure(&state, &workspace_id).await;

    let repo_path = crate::paths::expand_tilde(&repo.path);
    let job = LeadProvisionJob {
        workspace_id,
        thread_id: thread.id,
        repo_name: repo.name,
        setup_commands_raw: crate::harness_config::effective_setup(
            &repo.setup_commands,
            &repo_path,
        ),
        engine: thread.engine,
        goal,
        prompt: None,
        review: workspace.kind.as_deref() == Some("review"),
    };
    if let Err(error) = run_lead(&out, &state, job).await {
        log::warn!("retry provisioning failed: {error}");
    }
}

pub async fn skip_provisioning_setup(out: Output, state: AppState, workspace_id: String) {
    let Some((workspace, thread, repo)) = provisioning_rows(&state, &workspace_id) else {
        return;
    };
    let goal = state
        .store
        .first_message_text(&thread.id)
        .ok()
        .flatten()
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
        repo_name: repo.name,
        setup_commands_raw: String::new(),
        engine: thread.engine,
        goal,
        prompt: None,
        review: workspace.kind.as_deref() == Some("review"),
    };
    if let Err(error) = finish_after_setup(&out, &state, &job, 0).await {
        mark_failed(&out, &state, &workspace_id, None, None, &error.to_string()).await;
    }
}

async fn run_lead(out: &Output, state: &AppState, job: LeadProvisionJob) -> Result<()> {
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
    set_activity(state, &job.workspace_id, setup_activity, 1, None, None).await;
    emit_snapshot(out, state);

    if !worktree_exists(&record) {
        if let Err(error) = state.workspace.add_worktree(&job.workspace_id).await {
            mark_failed(
                out,
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
        if !run_setup_commands(out, state, &job, &record, &setup).await? {
            return Ok(());
        }
    } else {
        state.process.append_log_line(
            &job.workspace_id,
            "No setup commands configured for this repo",
        );
    }

    finish_after_setup(out, state, &job, setup.len()).await
}

async fn run_setup_commands(
    out: &Output,
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

        let code =
            match poll_session(&state.process, &session_id, log_id, SETUP_COMMAND_TIMEOUT).await {
                Ok(code) => code,
                Err(error) => {
                    mark_failed(
                        out,
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
            state
                .process
                .append_log_line(log_id, format!("Setup failed: exit code {code}"));
            mark_failed(
                out,
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
    out: &Output,
    state: &AppState,
    job: &LeadProvisionJob,
    setup_count: usize,
) -> Result<()> {
    if !workspace_alive(state, &job.workspace_id).await {
        return Ok(());
    }

    let start_agent = job.review || job.prompt.is_some() || !job.goal.trim().is_empty();
    if start_agent {
        set_activity(state, &job.workspace_id, "Starting agent…", 2, None, None).await;
        emit_snapshot(out, state);

        let prompt: Prompt = if job.review {
            crate::review::reviewer_engine_prompt(&job.goal).into()
        } else if let Some(prompt) = &job.prompt {
            prompt.clone()
        } else {
            job.goal.trim().into()
        };
        spawn_engine(
            state,
            &job.thread_id,
            &job.engine,
            &job.workspace_id,
            Some(prompt),
        )
        .await?;
    }

    let _ = record_step(
        &state.store,
        &job.thread_id,
        TimelineStep::WorktreeReady {
            repo: job.repo_name.clone(),
            setup_commands: setup_count as u32,
        },
    );

    if start_agent {
        seed_summary_if_needed(state, job).await?;
    }

    let thread_title = if job.review { "Reviewer" } else { "Lead" };
    let thread_status = if start_agent {
        ThreadStatus::Running
    } else {
        ThreadStatus::Idle
    };
    let _ = state.store.upsert_thread(&ThreadRow {
        id: job.thread_id.clone(),
        workspace_id: job.workspace_id.clone(),
        title: thread_title.into(),
        engine: job.engine.clone(),
        session_id: state.engines.session_id(&job.thread_id).ok().flatten(),
        status: thread_status.into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
        retired: false,
        parent_thread_id: None,
    });

    if start_agent {
        state
            .workspace
            .set_thread(&job.thread_id, &job.workspace_id, ThreadStatus::Running)
            .await;
        let record = state
            .workspace
            .set_status(&job.workspace_id, WorkspaceLifecycle::Running)
            .await?;
        set_activity(
            state,
            &job.workspace_id,
            initial_running_activity(job.review),
            0,
            None,
            None,
        )
        .await;
        persist_workspace_row(state, &job.workspace_id, "running", &record)?;
        state.notify(out, StateChangeKind::WorkspaceStatus);
        if job.review {
            emit_review_started_toast(out, state, &job.workspace_id, &job.thread_id);
        }
    } else {
        let record = state
            .workspace
            .set_status(&job.workspace_id, WorkspaceLifecycle::Ready)
            .await?;
        state
            .workspace
            .set_thread(&job.thread_id, &job.workspace_id, ThreadStatus::Idle)
            .await;
        let record = state
            .workspace
            .get(&job.workspace_id)
            .await
            .unwrap_or(record);
        set_activity(state, &job.workspace_id, "", 0, None, None).await;
        persist_workspace_row(state, &job.workspace_id, "idle", &record)?;
    }
    emit_snapshot(out, state);
    Ok(())
}

pub async fn ensure_thread_engine(
    state: &AppState,
    thread_id: &str,
    engine: &str,
    workspace_id: &str,
) -> Result<()> {
    if state.engines.has_thread(thread_id) {
        return Ok(());
    }
    // Two prompts can race here (a user message and a child's report); only one spawns.
    let _spawning = state.engines.lock_spawn(thread_id).await;
    if state.engines.has_thread(thread_id) {
        return Ok(());
    }
    spawn_engine(state, thread_id, engine, workspace_id, None).await
}

fn worktree_exists(record: &WorkspaceRecord) -> bool {
    Path::new(&record.worktree_path).exists()
}

async fn spawn_engine(
    state: &AppState,
    thread_id: &str,
    engine: &str,
    workspace_id: &str,
    prompt: Option<Prompt>,
) -> Result<()> {
    if state.workspace.get(workspace_id).await.is_none() {
        let row = state
            .store
            .workspace_by_id(workspace_id)?
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        if row.archived_at.is_some() {
            return Err(Error::Workspace(format!(
                "workspace {workspace_id} is archived"
            )));
        }
        let record = crate::git_workspace::resolve_record(state, workspace_id, &row).await?;
        state.workspace.remember(record).await;
    }
    let workspace = state
        .workspace
        .get(workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    if workspace.status == WorkspaceLifecycle::TearingDown {
        return Err(Error::Workspace(format!(
            "workspace {workspace_id} is being torn down"
        )));
    }
    let kind = EngineKind::from_name(engine);
    let run_everything = crate::approvals::run_everything_from_store(&state.store);
    state
        .engines
        .spawn(SpawnSpec {
            thread_id: thread_id.to_string(),
            kind,
            cwd: PathBuf::from(&workspace.worktree_path),
            resume: None,
            override_argv: None,
            auto_approve_readonly: false,
            auto_approve_all: run_everything,
            read_only: false,
        })
        .await?;
    if let Some(prompt) =
        prompt.filter(|prompt| !prompt.text.is_empty() || !prompt.images.is_empty())
    {
        state.engines.prompt(thread_id, prompt)?;
    }
    Ok(())
}

fn initial_running_activity(review: bool) -> &'static str {
    if review {
        "Reading the diff…"
    } else {
        "Reading repo…"
    }
}

async fn seed_summary_if_needed(state: &AppState, job: &LeadProvisionJob) -> Result<()> {
    let workspace = state
        .store
        .workspace_by_id(&job.workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", job.workspace_id)))?;
    if workspace
        .summary
        .as_ref()
        .is_some_and(|text| !text.is_empty())
    {
        return Ok(());
    }

    let record = state
        .workspace
        .get(&job.workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", job.workspace_id)))?;
    let summary = if job.review {
        let author = state
            .store
            .get_setting(&crate::review::review_author_key(&job.workspace_id))
            .ok()
            .flatten()
            .unwrap_or_else(|| "the author".into());
        let pr_label = workspace
            .pr_number
            .map(|num| format!("#{num}"))
            .unwrap_or_else(|| "the PR".into());
        format!(
            "Reviewing {pr_label} from {author} against {}. Claude Code is reading the diff and will draft comments for your approval; nothing has been posted to GitHub yet.",
            record.base
        )
    } else {
        format!(
            "Just started on “{}”, branched from {}. Claude Code is reading the repo to find where the change belongs; no files have been touched yet.",
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
        summary_at: Some(now),
        summary_source: workspace.summary_source,
        kind: workspace.kind,
        pr_number: workspace.pr_number,
        pr_html_url: workspace.pr_html_url.clone(),
        modified_files: workspace.modified_files,
        archived_at: workspace.archived_at,
    });
    Ok(())
}

async fn mark_failed(
    out: &Output,
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
    state.notify(out, StateChangeKind::WorkspaceStatus);
    emit_snapshot(out, state);
    out.toast(crate::ipc::types::ToastRaisedPayload {
        tone: crate::ipc::types::ToastTone::Bad,
        parts: vec![crate::ipc::types::ToastPart::Text {
            value: message.to_string(),
        }],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
    // Spawned: reporting can start an engine, and starting one can end up back here.
    let (out, state) = (out.clone(), state.clone());
    let workspace_id = workspace_id.to_string();
    let message = message.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(error) =
            crate::orchestration::on_provisioning_failed(&out, &state, &workspace_id, &message)
                .await
        {
            log::warn!("reporting failed setup: {error}");
        }
    });
}

/// The worktree is checked out, setup has run, and the Reviewer has its prompt.
fn emit_review_started_toast(out: &Output, state: &AppState, workspace_id: &str, thread_id: &str) {
    let pr_label = state
        .store
        .snapshot()
        .ok()
        .and_then(|snapshot| {
            snapshot
                .workspaces
                .into_iter()
                .find(|row| row.id == workspace_id)
                .and_then(|row| row.pr_number)
        })
        .map(|num| format!("#{num}"))
        .unwrap_or_else(|| "the PR".into());
    out.toast(crate::ipc::types::ToastRaisedPayload {
        tone: crate::ipc::types::ToastTone::Ok,
        parts: vec![
            crate::ipc::types::ToastPart::Text {
                value: "Reviewer started on ".into(),
            },
            crate::ipc::types::ToastPart::Code { value: pr_label },
        ],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: Some(thread_id.to_string()),
    });
}

async fn clear_failure(state: &AppState, workspace_id: &str) {
    set_activity(
        state,
        workspace_id,
        "Running worktree setup…",
        1,
        None,
        None,
    )
    .await;
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
        .set_provisioning_detail(workspace_id, activity, prov_step, failed_command, exit_code)
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

#[cfg(test)]
async fn wait_session(
    process: &ProcessSupervisor,
    session_id: &str,
    timeout: Duration,
) -> Result<i32> {
    poll_session(process, session_id, session_id, timeout).await
}

fn persist_workspace_status(state: &AppState, workspace_id: &str, status: &str) {
    if let Err(error) = state.store.set_workspace_status(workspace_id, status) {
        log::warn!("saving workspace status: {error}");
    }
}

/// The rows a retry or skip needs: the workspace, its Lead or Reviewer thread, and its repo.
fn provisioning_rows(
    state: &AppState,
    workspace_id: &str,
) -> Option<(WorkspaceRow, ThreadRow, crate::store::types::RepoRecord)> {
    let workspace = state
        .store
        .workspace_by_id(workspace_id)
        .map_err(|error| log::warn!("reading workspace {workspace_id}: {error}"))
        .ok()??;
    let thread = state
        .store
        .threads_for_workspace(workspace_id)
        .ok()?
        .into_iter()
        .find(|row| row.title == "Lead" || row.title == "Reviewer")?;
    let repo = state.store.repo_by_id(&workspace.repo_id).ok()??;
    Some((workspace, thread, repo))
}

fn persist_workspace_row(
    state: &AppState,
    workspace_id: &str,
    status: &str,
    record: &WorkspaceRecord,
) -> Result<()> {
    if state.store.workspace_by_id(workspace_id)?.is_none() {
        return Ok(());
    }
    state
        .store
        .set_workspace_checkout(workspace_id, &record.branch, &record.worktree_path)?;
    state.store.set_workspace_status(workspace_id, status)
}

fn emit_snapshot(out: &Output, state: &AppState) {
    state.notify(out, StateChangeKind::WorkspaceStatus);
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
    use crate::feedback::{Emitted, RecordingOutput};
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
        assert!(
            process
                .drain_pending(log_id)
                .iter()
                .any(|l| l.contains("ok"))
        );

        let fail_id = "test-ws-fail";
        process
            .spawn_session(fail_id, shell, &["-c", "exit 7"], None, &[])
            .unwrap();
        let code = wait_session(&process, fail_id, Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(code, 7);
    }

    #[tokio::test]
    async fn failed_setup_refreshes_toasts_and_then_merges_the_second_refresh() {
        let state = AppState::new(std::env::temp_dir());
        state.store.open_in_memory().unwrap();
        let (recording, out) = RecordingOutput::new();

        mark_failed(&out, &state, "w1", None, Some(1), "Setup broke").await;

        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(recording.toasts(), ["Setup broke"]);
        assert!(matches!(
            recording.emitted().as_slice(),
            [
                Emitted::State(StateChangeKind::WorkspaceStatus),
                Emitted::Toast(_),
                Emitted::State(StateChangeKind::WorkspaceStatus),
            ]
        ));
    }
}
