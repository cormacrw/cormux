use std::time::Instant;

use uuid::Uuid;

use crate::bus::DomainEvent;
use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::model::{ThreadStatus, WorkspaceKind, WorkspaceLifecycle};
use crate::paths::{expand_tilde, worktrees_root};
use crate::process::{Exit, SessionId, ShellCommand};
use crate::provisioning::{SETUP_COMMAND_TIMEOUT, parse_setup_commands};
use crate::state::AppState;
use crate::store::types::{ThreadRow, WorkspaceRow};
use crate::timeline::TimelineStep;
use crate::workspace_git::Checkout;
use crate::workspaces::WorkspaceRecord;

/// What a new workspace shows until its plan says otherwise.
pub const SETUP_ACTIVITY: &str = "Running worktree setup…";

/// A workspace to record: its row and its own thread.
#[derive(Debug, Clone)]
pub struct WorkspaceSpec {
    pub kind: WorkspaceKind,
    pub repo_id: String,
    pub name: String,
    pub branch: String,
    pub base: String,
    /// The pull request it's for, with its page.
    pub pr: Option<(i64, Option<String>)>,
    pub engine: String,
    /// The thread that asked for the workspace, when an agent did.
    pub parent_thread_id: Option<String>,
}

pub struct Created {
    pub workspace: WorkspaceRecord,
    pub thread_id: String,
}

type Intro = Box<dyn Fn(&WorkspaceRecord) -> String + Send + Sync>;

/// How a recorded workspace is brought up. Built by whoever knows what the workspace is for.
pub struct ProvisionPlan {
    pub workspace_id: String,
    pub thread_id: String,
    pub checkout: Checkout,
    /// Shown while the worktree is made and set up.
    pub setup_activity: String,
    /// The engine's first prompt. Without one the thread waits idle for the user.
    pub first_prompt: Option<Prompt>,
    /// Shown once the agent has its prompt.
    pub running_activity: String,
    /// What the workspace's intro says, from the workspace as setup leaves it.
    pub intro: Option<Intro>,
    /// Raised once the agent has its prompt.
    pub started_toast: Option<Vec<ToastPart>>,
}

/// Records the workspace and its thread, having checked everything that can be checked before
/// anything is written: the repo, the refs and the worktree folder. Nothing is notified.
pub async fn create(state: &AppState, spec: WorkspaceSpec) -> Result<Created> {
    let repo = state
        .store
        .repo_by_id(&spec.repo_id)?
        .ok_or_else(|| Error::unknown_repo(&spec.repo_id))?;
    repo.ensure_not_default_branch(&spec.branch)?;
    // Before any git call: `base` can come from an agent's `create_thread` call.
    crate::git::validate_ref_name(&spec.branch, "branch")?;
    crate::git::validate_ref_name(&spec.base, "base")?;
    let repo_path = expand_tilde(&repo.path);
    if !repo_path.is_dir() {
        return Err(Error::Git(format!(
            "repo path does not exist: {}",
            repo_path.display()
        )));
    }

    let workspace_id = format!("ws-{}", Uuid::new_v4().simple());
    let thread_id = Uuid::new_v4().to_string();
    let root = worktrees_root(&crate::settings::Settings::read(&state.store)?.worktree_root);
    let worktree = crate::paths::worktree_path(&root, &repo_path, &spec.branch);
    let mut row = WorkspaceRow::new(
        spec.kind,
        &workspace_id,
        &spec.repo_id,
        &spec.name,
        &spec.branch,
        &spec.base,
        worktree.to_string_lossy(),
    );
    if let Some((number, html_url)) = spec.pr {
        row = row.with_pr(number, html_url);
    }
    let thread = ThreadRow {
        parent_thread_id: spec.parent_thread_id,
        ..ThreadRow::new(
            &thread_id,
            &workspace_id,
            spec.kind.own_role(),
            &spec.engine,
        )
    };
    state.workspaces.register(row, &thread, &repo).await?;
    let workspace = state
        .workspaces
        .set_activity(&workspace_id, SETUP_ACTIVITY, 1, None)
        .await?;
    Ok(Created {
        workspace,
        thread_id,
    })
}

/// Runs the plan in the background.
pub fn start(state: &AppState, plan: ProvisionPlan) {
    let state = state.clone();
    tauri::async_runtime::spawn(async move { run(&state, plan).await });
}

/// Makes the worktree, runs the repo's setup commands and hands the thread its first turn. Any
/// error on the way fails the setup, so the workspace can be retried or skipped past.
pub async fn run(state: &AppState, plan: ProvisionPlan) {
    let ran = run_plan(state, &plan).await;
    fail_on_error(state, &plan.workspace_id, ran).await;
}

async fn fail_on_error(state: &AppState, workspace_id: &str, ran: Result<()>) {
    if let Err(error) = ran {
        log::warn!("provisioning {workspace_id} failed: {error}");
        mark_failed(state, workspace_id, None, &error.to_string()).await;
    }
}

/// Setup again, after it failed.
pub async fn retry(state: &AppState, plan: ProvisionPlan) -> Result<()> {
    state
        .workspaces
        .set_status(&plan.workspace_id, WorkspaceLifecycle::Provisioning)
        .await?;
    state
        .workspaces
        .set_activity(&plan.workspace_id, SETUP_ACTIVITY, 1, None)
        .await?;
    run(state, plan).await;
    Ok(())
}

/// Carries on past failed setup as if it had finished.
pub async fn skip_setup(state: &AppState, plan: ProvisionPlan) -> Result<()> {
    state
        .workspaces
        .set_status(&plan.workspace_id, WorkspaceLifecycle::Provisioning)
        .await?;
    state
        .workspaces
        .set_activity(&plan.workspace_id, SETUP_ACTIVITY, 1, None)
        .await?;
    state.process.append_log_line(
        &plan.workspace_id,
        "──── Skipped remaining setup — continuing ────",
    );
    let finished = finish_after_setup(state, &plan, 0).await;
    fail_on_error(state, &plan.workspace_id, finished).await;
    Ok(())
}

async fn run_plan(state: &AppState, plan: &ProvisionPlan) -> Result<()> {
    if !workspace_alive(state, &plan.workspace_id).await {
        return Ok(());
    }
    let workspace = state.workspaces.get(&plan.workspace_id).await?;
    let repo = state
        .store
        .repo_by_id(&workspace.repo_id)?
        .ok_or_else(|| Error::unknown_repo(&workspace.repo_id))?;
    let setup = match crate::harness_config::effective_setup(
        &repo.setup_commands,
        &expand_tilde(&repo.path),
    ) {
        Ok(setup) => parse_setup_commands(&setup),
        Err(error) => {
            log::warn!("setup for {}: {error}", plan.workspace_id);
            mark_failed(state, &plan.workspace_id, None, &error.to_string()).await;
            return Ok(());
        }
    };

    let workspace = state
        .workspaces
        .set_activity(&plan.workspace_id, &plan.setup_activity, 1, None)
        .await?;
    state.out.changed();

    if !workspace.worktree().exists() {
        let added = crate::workspace_git::add_worktree(state, &workspace, plan.checkout).await;
        if let Err(error) = added {
            mark_failed(state, &plan.workspace_id, None, &error.to_string()).await;
            return Ok(());
        }
        if !workspace_alive(state, &plan.workspace_id).await {
            return Ok(());
        }
    }
    crate::changes::watch(state, &workspace)?;

    if setup.is_empty() {
        state.process.append_log_line(
            &plan.workspace_id,
            "No setup commands configured for this repo",
        );
    } else if !run_setup_commands(state, &workspace, &repo.name, &setup).await? {
        return Ok(());
    }

    finish_after_setup(state, plan, setup.len()).await
}

async fn run_setup_commands(
    state: &AppState,
    workspace: &WorkspaceRecord,
    repo_name: &str,
    setup: &[String],
) -> Result<bool> {
    let log_id = &workspace.id;
    state.process.append_log_line(
        log_id,
        format!(
            "Running {} setup command{} from {repo_name} settings",
            setup.len(),
            if setup.len() == 1 { "" } else { "s" },
        ),
    );

    let env = workspace.harness_env().vars();

    for (index, command) in setup.iter().enumerate() {
        if !workspace_alive(state, &workspace.id).await {
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

        let session = SessionId::Setup {
            workspace: workspace.id.clone(),
            index,
        };
        let shell = ShellCommand {
            cmd: command.clone(),
            cwd: workspace.worktree(),
            env: env.clone(),
        };
        let exit = state
            .process
            .run_to_exit(session, &shell, SETUP_COMMAND_TIMEOUT, Some(log_id))
            .await?;
        let Exit::Code(code) = exit else {
            let failure = Some((command.clone(), -1));
            let error = Error::Process("timed out waiting for setup command".into());
            mark_failed(state, &workspace.id, failure, &error.to_string()).await;
            return Ok(false);
        };

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
                state,
                &workspace.id,
                Some((command.clone(), code)),
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
    state: &AppState,
    plan: &ProvisionPlan,
    setup_count: usize,
) -> Result<()> {
    if !workspace_alive(state, &plan.workspace_id).await {
        return Ok(());
    }
    let id = &plan.workspace_id;

    if let Some(prompt) = &plan.first_prompt {
        state
            .workspaces
            .set_activity(id, "Starting agent…", 2, None)
            .await?;
        state.out.changed();
        crate::turn::begin(state, &plan.thread_id, id, prompt.clone()).await?;
    }

    let workspace = state.workspaces.get(id).await?;
    let repo_name = state
        .store
        .repo_by_id(&workspace.repo_id)?
        .map(|repo| repo.name)
        .unwrap_or_default();
    if let Err(error) = state.transcript.record_step(
        &plan.thread_id,
        TimelineStep::WorktreeReady {
            repo: repo_name,
            setup_commands: setup_count as u32,
        },
    ) {
        log::warn!("recording the worktree step: {error}");
    }

    if plan.first_prompt.is_some() {
        if let Some(intro) = &plan.intro {
            write_intro(state, &workspace, intro)?;
        }
        state
            .workspaces
            .set_thread_status(&plan.thread_id, id, ThreadStatus::Running)
            .await?;
        state
            .workspaces
            .set_status(id, WorkspaceLifecycle::Running)
            .await?;
        state
            .workspaces
            .set_activity(id, &plan.running_activity, 0, None)
            .await?;
        state.out.changed();
        if let Some(parts) = &plan.started_toast {
            state.out.toast(ToastRaisedPayload {
                tone: ToastTone::Ok,
                parts: parts.clone(),
                workspace_id: Some(id.clone()),
                thread_id: Some(plan.thread_id.clone()),
            });
        }
    } else {
        state
            .workspaces
            .set_status(id, WorkspaceLifecycle::Ready)
            .await?;
        state
            .workspaces
            .set_thread_status(&plan.thread_id, id, ThreadStatus::Idle)
            .await?;
        state.workspaces.set_activity(id, "", 0, None).await?;
    }
    state.out.changed();
    Ok(())
}

/// Written once: a retried setup keeps the first.
fn write_intro(state: &AppState, workspace: &WorkspaceRecord, intro: &Intro) -> Result<()> {
    let written = state
        .store
        .workspace_by_id(&workspace.id)?
        .and_then(|row| row.summary)
        .is_some_and(|text| !text.is_empty());
    if !written {
        state
            .workspaces
            .set_summary(&workspace.id, &intro(workspace))?;
    }
    Ok(())
}

/// Setup stopped: says why and reports it. A workspace being torn down stopped its own setup,
/// so it's left alone.
async fn mark_failed(
    state: &AppState,
    workspace_id: &str,
    failure: Option<(String, i32)>,
    message: &str,
) {
    if !workspace_alive(state, workspace_id).await {
        return;
    }
    let updated = state
        .workspaces
        .set_activity(workspace_id, "Setup failed", 0, failure)
        .await;
    let failed = state
        .workspaces
        .set_status(workspace_id, WorkspaceLifecycle::ProvisioningFailed)
        .await;
    if let Err(error) = updated.and(failed) {
        log::warn!("marking {workspace_id} failed: {error}");
    }
    state.out.changed();
    state.out.toast(ToastRaisedPayload {
        tone: ToastTone::Bad,
        parts: vec![ToastPart::Text {
            value: message.to_string(),
        }],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
    state.bus.publish(DomainEvent::ProvisioningFailed {
        workspace_id: workspace_id.to_string(),
        message: message.to_string(),
    });
}

async fn workspace_alive(state: &AppState, workspace_id: &str) -> bool {
    state
        .workspaces
        .get(workspace_id)
        .await
        .is_ok_and(|record| record.status != WorkspaceLifecycle::TearingDown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::Emitted;
    use crate::store::types::RepoRecord;

    async fn provisioning_workspace() -> (
        AppState,
        std::sync::Arc<crate::feedback::RecordingOutput>,
        tempfile::TempDir,
        String,
    ) {
        let (state, recording, _fake, dir, id) = provisioning_workspace_faked().await;
        (state, recording, dir, id)
    }

    async fn provisioning_workspace_faked() -> (
        AppState,
        std::sync::Arc<crate::feedback::RecordingOutput>,
        crate::thread_runtime::fake::FakeEngines,
        tempfile::TempDir,
        String,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("app");
        std::fs::create_dir(&repo).unwrap();
        let (state, recording, fake) =
            AppState::faked(crate::paths::DataDir::new(dir.path().join("data")));
        state
            .store
            .upsert_repo(&RepoRecord {
                path: repo.to_string_lossy().to_string(),
                name: "app".into(),
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
        let created = create(
            &state,
            WorkspaceSpec {
                kind: WorkspaceKind::Lead,
                repo_id: "r1".into(),
                name: "Work".into(),
                branch: "feat/x".into(),
                base: "develop".into(),
                pr: None,
                engine: "claude".into(),
                parent_thread_id: None,
            },
        )
        .await
        .unwrap();
        (state, recording, fake, dir, created.workspace.id)
    }

    #[tokio::test]
    async fn a_created_workspace_keeps_its_base_and_gets_an_uncollidable_id() {
        let (state, _recording, _dir, id) = provisioning_workspace().await;
        assert!(id.starts_with("ws-") && id.len() > 30, "{id}");
        let row = state.store.workspace_by_id(&id).unwrap().unwrap();
        assert_eq!(row.base, "develop");
        assert_eq!(row.status, WorkspaceLifecycle::Provisioning);
        let thread = state.store.own_thread(&id).unwrap().unwrap();
        assert_eq!(thread.role, Some(crate::model::ThreadRole::Lead));
        assert_eq!(thread.title, "Lead");
    }

    #[tokio::test]
    async fn failed_setup_refreshes_then_toasts() {
        let (state, recording, _dir, id) = provisioning_workspace().await;

        mark_failed(&state, &id, None, "Setup broke").await;

        assert_eq!(recording.toasts(), ["Setup broke"]);
        assert!(matches!(
            recording.emitted().as_slice(),
            [Emitted::State(_), Emitted::Toast(_)]
        ));
        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::ProvisioningFailed
        );
    }

    #[tokio::test]
    async fn setup_killed_by_teardown_is_not_a_failure() {
        let (state, recording, _dir, id) = provisioning_workspace().await;
        state
            .workspaces
            .set_status(&id, WorkspaceLifecycle::TearingDown)
            .await
            .unwrap();

        mark_failed(&state, &id, Some(("pnpm install".into(), 143)), "killed").await;

        assert!(recording.emitted().is_empty());
        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::TearingDown
        );
    }

    #[tokio::test]
    async fn a_first_turn_that_cant_start_fails_the_setup() {
        let (state, recording, fake, _dir, id) = provisioning_workspace_faked().await;
        let workspace = state.workspaces.get(&id).await.unwrap();
        std::fs::create_dir_all(workspace.worktree()).unwrap();
        let thread_id = state.store.own_thread(&id).unwrap().unwrap().id;
        fake.refuse_sends(&thread_id);

        run(
            &state,
            ProvisionPlan {
                workspace_id: id.clone(),
                thread_id,
                checkout: Checkout::NewBranch,
                setup_activity: SETUP_ACTIVITY.into(),
                first_prompt: Some("Go".into()),
                running_activity: "Working".into(),
                intro: None,
                started_toast: None,
            },
        )
        .await;

        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::ProvisioningFailed
        );
        assert_eq!(recording.toasts().len(), 1);
    }
}
