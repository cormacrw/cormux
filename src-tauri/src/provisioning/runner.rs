use std::time::Instant;

use uuid::Uuid;

use crate::bus::DomainEvent;
use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::model::{ThreadStatus, WorkspaceKind, WorkspaceLifecycle};
use crate::paths::{expand_tilde, worktrees_root};
use crate::process::{Exit, LogKind, SessionId, ShellCommand};
use crate::provisioning::{SETUP_COMMAND_TIMEOUT, parse_setup_commands};
use crate::state::AppState;
use crate::store::types::{ThreadPicks, ThreadRow, WorkspaceRow};
use crate::timeline::TimelineStep;
use crate::workspace_git::Checkout;
use crate::workspaces::{WorkspaceActivity, WorkspaceRecord};

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
    /// What its thread starts on, stored with it, since the engine reads it when it launches.
    pub picks: ThreadPicks,
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
    /// The engine's first prompt. Without one the thread waits idle for the user.
    pub first_prompt: Option<Prompt>,
    /// What the workspace's intro says, from the workspace as setup leaves it.
    pub intro: Option<Intro>,
    /// Raised once the agent has its prompt.
    pub started_toast: Option<Vec<ToastPart>>,
}

/// Records the workspace and its thread, having checked everything that can be checked before
/// anything is written: the repo, the refs and the worktree folder. Nothing is notified.
pub async fn create(state: &AppState, spec: WorkspaceSpec) -> Result<Created> {
    let repo = state.store.repo(&spec.repo_id)?;
    // Before any git call: `base` can come from an agent's `create_thread` call.
    crate::workspace_git::check_branch_name(&repo, &spec.branch)?;
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
    state
        .workspaces
        .register(row, &thread, &spec.picks, &repo)
        .await?;
    let workspace = state
        .workspaces
        .set_activity(&workspace_id, WorkspaceActivity::SettingUp)
        .await?;
    Ok(Created {
        workspace,
        thread_id,
    })
}

pub fn start(state: &AppState, plan: ProvisionPlan) {
    let state = state.clone();
    tauri::async_runtime::spawn(async move { run(&state, plan).await });
}

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

/// Runs a failed setup again in the background. Refused unless setup failed.
pub async fn retry(state: &AppState, plan: ProvisionPlan) -> Result<()> {
    state
        .workspaces
        .restart_provisioning(&plan.workspace_id)
        .await?;
    state.out.changed();
    start(state, plan);
    Ok(())
}

/// Carries on past failed setup as if it had finished, in the background. Refused unless
/// setup failed after making the worktree, which the agent needs to start in.
pub async fn skip_setup(state: &AppState, plan: ProvisionPlan) -> Result<()> {
    let workspace = state.workspaces.get(&plan.workspace_id).await?;
    if !workspace.worktree().exists() {
        return Err(Error::Workspace(
            "The worktree was never made, so setup can't be skipped. Retry it instead.".into(),
        ));
    }
    state
        .workspaces
        .restart_provisioning(&plan.workspace_id)
        .await?;
    state.out.changed();
    state.process.log(
        &plan.workspace_id,
        LogKind::Marker,
        "Skipped remaining setup — continuing",
    );
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        let finished = finish_after_setup(&state, &plan, 0).await;
        fail_on_error(&state, &plan.workspace_id, finished).await;
    });
    Ok(())
}

async fn run_plan(state: &AppState, plan: &ProvisionPlan) -> Result<()> {
    if !workspace_alive(state, &plan.workspace_id).await {
        return Ok(());
    }
    let workspace = state.workspaces.get(&plan.workspace_id).await?;
    let repo = state.store.repo(&workspace.repo_id)?;
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
        .set_activity(&plan.workspace_id, WorkspaceActivity::SettingUp)
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
        state.process.log(
            &plan.workspace_id,
            LogKind::Note,
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
    state.process.log(
        log_id,
        LogKind::Note,
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
            .log(log_id, LogKind::Command, command.as_str());
        state.process.log(
            log_id,
            LogKind::Note,
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
            let failure = Some((command.clone(), None));
            let error = Error::Process("timed out waiting for setup command".into());
            mark_failed(state, &workspace.id, failure, &error.to_string()).await;
            return Ok(false);
        };

        state.process.log(
            log_id,
            LogKind::Note,
            format!(
                "Finished command {} of {} in {:.1}s",
                index + 1,
                setup.len(),
                started.elapsed().as_secs_f32(),
            ),
        );

        if code != 0 {
            state.process.log(
                log_id,
                LogKind::Note,
                format!("Setup failed: exit code {code}"),
            );
            mark_failed(
                state,
                &workspace.id,
                Some((command.clone(), Some(code))),
                &format!("Setup command failed with exit code {code}"),
            )
            .await;
            return Ok(false);
        }
    }

    state.process.log(log_id, LogKind::Marker, "Worktree ready");
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
            .set_activity(id, WorkspaceActivity::StartingAgent)
            .await?;
        state.out.changed();
        crate::turn::begin(state, &plan.thread_id, id, prompt.clone()).await?;
        // At once, so a turn that ends quickly isn't marked running after it settled.
        state
            .workspaces
            .finish_provisioning(
                id,
                &plan.thread_id,
                ThreadStatus::Running,
                WorkspaceLifecycle::Running,
                WorkspaceActivity::AgentStarted,
            )
            .await?;
        state.out.changed();
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
        if let Some(parts) = &plan.started_toast {
            state.out.toast(ToastRaisedPayload {
                tone: ToastTone::Ok,
                parts: parts.clone(),
                workspace_id: Some(id.clone()),
                thread_id: Some(plan.thread_id.clone()),
                ..Default::default()
            });
        }
    } else {
        state
            .workspaces
            .finish_provisioning(
                id,
                &plan.thread_id,
                ThreadStatus::Idle,
                WorkspaceLifecycle::Ready,
                WorkspaceActivity::None,
            )
            .await?;
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

/// Fails a recorded workspace's setup before it ran, so it can be retried or torn down.
pub async fn fail(state: &AppState, workspace_id: &str, message: &str) {
    mark_failed(state, workspace_id, None, message).await;
}

/// A workspace being torn down stopped its own setup, so it's left alone.
async fn mark_failed(
    state: &AppState,
    workspace_id: &str,
    failure: Option<(String, Option<i32>)>,
    message: &str,
) {
    if !workspace_alive(state, workspace_id).await {
        return;
    }
    let updated = state
        .workspaces
        .set_activity(
            workspace_id,
            WorkspaceActivity::SetupFailed {
                exit_code: failure.as_ref().and_then(|(_, code)| *code),
                command: failure.map(|(command, _)| command),
            },
        )
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
        ..Default::default()
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
                picks: Default::default(),
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
        assert_eq!(
            state.workspaces.get(&id).await.unwrap().activity,
            WorkspaceActivity::SettingUp
        );

        mark_failed(&state, &id, Some(("pnpm i".into(), Some(1))), "Setup broke").await;

        assert_eq!(recording.toasts(), ["Setup broke"]);
        assert!(matches!(
            recording.emitted().as_slice(),
            [Emitted::State(_), Emitted::Toast(_)]
        ));
        let failed = state.workspaces.get(&id).await.unwrap();
        assert_eq!(failed.status, WorkspaceLifecycle::ProvisioningFailed);
        assert_eq!(
            failed.activity,
            WorkspaceActivity::SetupFailed {
                command: Some("pnpm i".into()),
                exit_code: Some(1),
            }
        );
        assert_eq!(
            serde_json::to_value(&failed.activity).unwrap(),
            serde_json::json!({ "kind": "setupFailed", "command": "pnpm i", "exitCode": 1 })
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

        mark_failed(
            &state,
            &id,
            Some(("pnpm install".into(), Some(143))),
            "killed",
        )
        .await;

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
                first_prompt: Some("Go".into()),
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

    fn plan(id: &str, thread_id: &str, prompt: Option<&str>) -> ProvisionPlan {
        ProvisionPlan {
            workspace_id: id.into(),
            thread_id: thread_id.into(),
            checkout: Checkout::NewBranch,
            first_prompt: prompt.map(Into::into),
            intro: None,
            started_toast: None,
        }
    }

    async fn with_worktree() -> (AppState, tempfile::TempDir, String, String) {
        let (state, _recording, _fake, dir, id) = provisioning_workspace_faked().await;
        let workspace = state.workspaces.get(&id).await.unwrap();
        std::fs::create_dir_all(workspace.worktree()).unwrap();
        let thread_id = state.store.own_thread(&id).unwrap().unwrap().id;
        (state, dir, id, thread_id)
    }

    /// Bug: setting the thread idle after the workspace re-derived its status, so a workspace
    /// set up with no prompt read as idle rather than ready.
    #[tokio::test]
    async fn setup_ends_ready_with_no_prompt_and_running_with_one() {
        for (prompt, thread, lifecycle) in [
            (None, ThreadStatus::Idle, WorkspaceLifecycle::Ready),
            (
                Some("Go"),
                ThreadStatus::Running,
                WorkspaceLifecycle::Running,
            ),
        ] {
            let (state, _dir, id, thread_id) = with_worktree().await;

            run(&state, plan(&id, &thread_id, prompt)).await;

            let record = state.workspaces.get(&id).await.unwrap();
            assert_eq!(record.status, lifecycle, "{prompt:?}");
            let row = state.store.workspace_by_id(&id).unwrap().unwrap();
            assert_eq!(row.status, lifecycle, "{prompt:?}");
            let own = state.store.own_thread(&id).unwrap().unwrap();
            assert_eq!(own.status, thread, "{prompt:?}");
        }
    }

    /// Bug: Retry and Skip ran whatever the status, so a second click started a second run
    /// over the first, and Skip started an agent in a worktree that was never made.
    #[tokio::test]
    async fn retry_and_skip_only_take_a_failed_setup() {
        let (state, _dir, id, thread_id) = with_worktree().await;
        let refused = retry(&state, plan(&id, &thread_id, None))
            .await
            .unwrap_err();
        assert!(refused.to_string().contains("isn't failed"), "{refused}");

        mark_failed(&state, &id, None, "broke").await;
        retry(&state, plan(&id, &thread_id, None)).await.unwrap();
        assert!(retry(&state, plan(&id, &thread_id, None)).await.is_err());
        assert!(
            skip_setup(&state, plan(&id, &thread_id, None))
                .await
                .is_err()
        );

        let (state, _recording, _dir, id) = provisioning_workspace().await;
        let thread_id = state.store.own_thread(&id).unwrap().unwrap().id;
        mark_failed(&state, &id, None, "no worktree").await;
        let refused = skip_setup(&state, plan(&id, &thread_id, None))
            .await
            .unwrap_err();
        assert!(refused.to_string().contains("never made"), "{refused}");
        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::ProvisioningFailed
        );
    }
}
