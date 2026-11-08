use std::time::Instant;

use uuid::Uuid;

use crate::bus::DomainEvent;
use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::harness_config::{SetupEnd, SetupWatch, run_setup};
use crate::model::{ThreadStatus, WorkspaceKind, WorkspaceLifecycle};
use crate::paths::{expand_tilde, worktrees_root};
use crate::process::{Exit, LogKind, SessionId};
use crate::provisioning::SetupRun;
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
    /// The thread that asked for the workspace, when an agent did.
    pub parent_thread_id: Option<String>,
    /// What its thread starts on, stored with it, since the engine reads it when it launches.
    pub picks: ThreadPicks,
    /// The message its first turn starts from, kept so a retried setup starts from it again.
    pub setup_message: String,
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
/// anything is written: the repo, the refs and the worktree folder. Then `first_records`, the
/// caller's own first writes for it; once the workspace is recorded, a step that fails fails its
/// setup, leaving it to retry or tear down. Nothing else is notified.
pub async fn create(
    state: &AppState,
    spec: WorkspaceSpec,
    first_records: impl FnOnce(&Created) -> Result<()>,
) -> Result<Created> {
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
            crate::settings::Settings::load(&state.store).default_engine_name(),
        )
    };
    state
        .workspaces
        .register(row, &thread, &spec.picks, &spec.setup_message, &repo)
        .await?;
    let created = state
        .workspaces
        .set_activity(&workspace_id, WorkspaceActivity::SettingUp)
        .await
        .map(|workspace| Created {
            workspace,
            thread_id,
        })
        .and_then(|created| first_records(&created).map(|()| created));
    if let Err(error) = &created {
        mark_failed(state, &workspace_id, None, None, &error.to_string()).await;
    }
    created
}

pub fn start(state: &AppState, plan: ProvisionPlan) {
    let state = state.clone();
    tauri::async_runtime::spawn(async move { run(&state, plan).await });
}

pub async fn run(state: &AppState, plan: ProvisionPlan) {
    let run = state.setup.begin(&plan.workspace_id).await;
    let ran = run_plan(state, &plan, &run).await;
    fail_on_error(state, &run, ran).await;
}

async fn fail_on_error(state: &AppState, run: &SetupRun, ran: Result<()>) {
    if let Err(error) = ran {
        log::warn!("provisioning {} failed: {error}", run.workspace_id);
        mark_failed(
            state,
            &run.workspace_id,
            Some(run),
            None,
            &error.to_string(),
        )
        .await;
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
        let run = state.setup.begin(&plan.workspace_id).await;
        let finished = finish_after_setup(&state, &plan, &run, 0).await;
        fail_on_error(&state, &run, finished).await;
    });
    Ok(())
}

async fn run_plan(state: &AppState, plan: &ProvisionPlan, run: &SetupRun) -> Result<()> {
    if !alive(state, run).await {
        return Ok(());
    }
    let workspace = state.workspaces.get(&plan.workspace_id).await?;
    let repo = state.store.repo(&workspace.repo_id)?;
    let setup = match crate::harness_config::setup_commands(&repo) {
        Ok(setup) => setup,
        Err(error) => {
            log::warn!("setup for {}: {error}", plan.workspace_id);
            mark_failed(
                state,
                &run.workspace_id,
                Some(run),
                None,
                &error.to_string(),
            )
            .await;
            return Ok(());
        }
    };

    let workspace = state
        .workspaces
        .set_activity(&plan.workspace_id, WorkspaceActivity::SettingUp)
        .await?;
    state.out.changed();

    if let Err(error) = make_worktree(state, plan, run, &workspace).await {
        mark_failed(
            state,
            &run.workspace_id,
            Some(run),
            None,
            &error.to_string(),
        )
        .await;
        return Ok(());
    }
    if !alive(state, run).await {
        return Ok(());
    }

    if setup.is_empty() {
        state.process.log(
            &plan.workspace_id,
            LogKind::Note,
            "No setup commands configured for this repo",
        );
    } else if !run_setup_commands(state, run, &workspace, &repo.name, &setup).await? {
        return Ok(());
    }

    finish_after_setup(state, plan, run, setup.len()).await
}

/// One step teardown waits for: the worktree is there and watched once it's done, unless the
/// run was over before it began. A new branch's thread says so once git has made it; a PR's
/// review says what it checked out itself.
async fn make_worktree(
    state: &AppState,
    plan: &ProvisionPlan,
    run: &SetupRun,
    workspace: &WorkspaceRecord,
) -> Result<()> {
    let Some(_step) = state.setup.step(run).await else {
        return Ok(());
    };
    if !workspace.worktree().exists() {
        crate::workspace_git::add_worktree(state, workspace, plan.checkout).await?;
        if let Checkout::NewBranch = plan.checkout {
            let step = TimelineStep::WorktreeCreated {
                branch: workspace.branch.clone(),
                base: workspace.base.clone(),
            };
            if let Err(error) = state.transcript.record_step(&plan.thread_id, step) {
                log::warn!("could not record the worktree step: {error}");
            }
        }
    }
    crate::changes::watch(state, workspace)
}

async fn run_setup_commands(
    state: &AppState,
    run: &SetupRun,
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
    let mut watch = SetupLog {
        state,
        run,
        workspace_id: log_id,
        count: setup.len(),
        started: Instant::now(),
    };
    let ended = run_setup(
        &state.process,
        setup,
        &workspace.worktree(),
        &env,
        Some(log_id),
        &mut watch,
    )
    .await?;
    match ended {
        SetupEnd::Done => {}
        SetupEnd::Stopped => return Ok(false),
        SetupEnd::Failed {
            command,
            exit: Exit::TimedOut,
        } => {
            let error = Error::Process("timed out waiting for setup command".into());
            mark_failed(
                state,
                &run.workspace_id,
                Some(run),
                Some((command, None)),
                &error.to_string(),
            )
            .await;
            return Ok(false);
        }
        SetupEnd::Failed {
            command,
            exit: Exit::Code(code),
        } => {
            state.process.log(
                log_id,
                LogKind::Note,
                format!("Setup failed: exit code {code}"),
            );
            mark_failed(
                state,
                &workspace.id,
                Some(run),
                Some((command, Some(code))),
                &format!("Setup command failed with exit code {code}"),
            )
            .await;
            return Ok(false);
        }
    }

    state.process.log(log_id, LogKind::Marker, "Worktree ready");
    Ok(true)
}

/// Setup's own log around each command, and its run's liveness.
struct SetupLog<'a> {
    state: &'a AppState,
    run: &'a SetupRun,
    workspace_id: &'a str,
    count: usize,
    started: Instant,
}

impl SetupWatch for SetupLog<'_> {
    fn session(&self, index: usize) -> SessionId {
        SessionId::of_workspace(self.workspace_id, format!("setup-{index}"))
    }

    async fn before(&mut self, index: usize, command: &str) -> bool {
        if !alive(self.state, self.run).await {
            return false;
        }
        self.started = Instant::now();
        let process = &self.state.process;
        process.log(self.workspace_id, LogKind::Command, command);
        process.log(
            self.workspace_id,
            LogKind::Note,
            format!("── Command {} of {} ──", index + 1, self.count),
        );
        true
    }

    fn exited(&mut self, index: usize, exit: &Exit) {
        if let Exit::Code(_) = exit {
            self.state.process.log(
                self.workspace_id,
                LogKind::Note,
                format!(
                    "Finished command {} of {} in {:.1}s",
                    index + 1,
                    self.count,
                    self.started.elapsed().as_secs_f32(),
                ),
            );
        }
    }
}

async fn finish_after_setup(
    state: &AppState,
    plan: &ProvisionPlan,
    run: &SetupRun,
    setup_count: usize,
) -> Result<()> {
    if !alive(state, run).await {
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

/// Only a workspace still setting up fails: one being torn down stopped its own setup, and a run
/// that teardown cancelled or a newer run replaced is left alone.
async fn mark_failed(
    state: &AppState,
    workspace_id: &str,
    run: Option<&SetupRun>,
    failure: Option<(String, Option<i32>)>,
    message: &str,
) {
    if let Some(run) = run
        && state.setup.step(run).await.is_none()
    {
        return;
    }
    let failed = state
        .workspaces
        .fail_provisioning(
            workspace_id,
            WorkspaceActivity::SetupFailed {
                exit_code: failure.as_ref().and_then(|(_, code)| *code),
                command: failure.map(|(command, _)| command),
            },
        )
        .await;
    match failed {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => log::warn!("marking {workspace_id} failed: {error}"),
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

/// The run is the workspace's newest and teardown hasn't cancelled it, and the workspace is
/// still setting up: not failed, not being torn down.
async fn alive(state: &AppState, run: &SetupRun) -> bool {
    state.setup.step(run).await.is_some()
        && state
            .workspaces
            .get(&run.workspace_id)
            .await
            .is_ok_and(|record| record.status == WorkspaceLifecycle::Provisioning)
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
        let (state, recording, fake, dir, created) = creating("", |_| Ok(())).await;
        (state, recording, fake, dir, created.unwrap().workspace.id)
    }

    async fn creating(
        setup_message: &str,
        first_records: impl FnOnce(&Created) -> Result<()>,
    ) -> (
        AppState,
        std::sync::Arc<crate::feedback::RecordingOutput>,
        crate::thread_runtime::fake::FakeEngines,
        tempfile::TempDir,
        Result<Created>,
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
                parent_thread_id: None,
                picks: Default::default(),
                setup_message: setup_message.into(),
            },
            first_records,
        )
        .await;
        (state, recording, fake, dir, created)
    }

    #[tokio::test]
    async fn a_workspace_is_recorded_with_the_message_it_starts_from() {
        let (state, _recording, _fake, _dir, created) = creating("Fix it", |_| Ok(())).await;
        let id = created.unwrap().workspace.id;
        assert_eq!(
            state.store.setup_message(&id).unwrap().as_deref(),
            Some("Fix it")
        );
    }

    /// Bug: a review whose first records failed to write was left provisioning with no setup
    /// run, so neither Retry nor Skip took it.
    #[tokio::test]
    async fn a_first_record_that_fails_fails_the_setup() {
        let (state, recording, _fake, _dir, created) =
            creating("Review it", |_| Err(Error::Workspace("disk full".into()))).await;

        assert!(created.is_err());
        let row = state.store.live_workspaces().unwrap().pop().unwrap();
        assert_eq!(row.status, WorkspaceLifecycle::ProvisioningFailed);
        assert_eq!(
            state.store.setup_message(&row.id).unwrap().as_deref(),
            Some("Review it")
        );
        assert_eq!(recording.toasts(), ["workspace: disk full"]);
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

        mark_failed(
            &state,
            &id,
            None,
            Some(("pnpm i".into(), Some(1))),
            "Setup broke",
        )
        .await;

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
            None,
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

    /// Bug: the step was recorded before setup ran, so a worktree git couldn't make still read
    /// as created (and one a retry made was never recorded).
    #[tokio::test]
    async fn a_worktree_git_cannot_make_is_not_recorded_as_created() {
        let (state, _recording, _dir, id) = provisioning_workspace().await;
        let thread_id = state.store.own_thread(&id).unwrap().unwrap().id;

        run(&state, plan(&id, &thread_id, None)).await;

        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::ProvisioningFailed
        );
        let steps = state
            .store
            .thread_event_page(&thread_id, None, None, None)
            .unwrap()
            .events;
        assert!(
            !steps
                .iter()
                .any(|event| event.payload.contains("worktreeCreated")),
            "{steps:?}"
        );
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

        mark_failed(&state, &id, None, None, "broke").await;
        retry(&state, plan(&id, &thread_id, None)).await.unwrap();
        assert!(retry(&state, plan(&id, &thread_id, None)).await.is_err());
        assert!(
            skip_setup(&state, plan(&id, &thread_id, None))
                .await
                .is_err()
        );

        let (state, _recording, _dir, id) = provisioning_workspace().await;
        let thread_id = state.store.own_thread(&id).unwrap().unwrap().id;
        mark_failed(&state, &id, None, None, "no worktree").await;
        let refused = skip_setup(&state, plan(&id, &thread_id, None))
            .await
            .unwrap_err();
        assert!(refused.to_string().contains("never made"), "{refused}");
        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::ProvisioningFailed
        );
    }

    /// Bug: teardown didn't wait for setup to finish adding the worktree, so it archived the
    /// workspace and left the worktree and branch behind once `worktree add` returned.
    #[tokio::test]
    async fn teardown_waits_for_the_worktree_step_and_the_run_ends_there() {
        let (state, _dir, id, _thread_id) = with_worktree().await;
        let run = state.setup.begin(&id).await;
        let step = state.setup.step(&run).await.unwrap();

        let cancelling = {
            let state = state.clone();
            let id = id.clone();
            tokio::spawn(async move {
                drop(state.setup.cancel(&id).await);
            })
        };
        // One thread runs the test, so this lets the cancel run until it waits on the step.
        tokio::task::yield_now().await;
        assert!(!cancelling.is_finished(), "the step is still going");

        drop(step);
        cancelling.await.unwrap();
        assert!(state.setup.step(&run).await.is_none());
    }

    /// Bug: a teardown that failed handed the workspace back as failed setup, which setup read
    /// as alive, so it ran its commands and started the agent in a workspace the user deleted.
    #[tokio::test]
    async fn a_run_teardown_cancelled_stays_stopped_after_the_teardown_fails() {
        let (state, _dir, id, thread_id) = with_worktree().await;
        let run = state.setup.begin(&id).await;
        drop(state.setup.cancel(&id).await);
        mark_failed(&state, &id, None, None, "teardown failed").await;

        finish_after_setup(&state, &plan(&id, &thread_id, Some("Go")), &run, 0)
            .await
            .unwrap();

        assert!(!state.engines.has_thread(&thread_id));
        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::ProvisioningFailed
        );
    }

    #[tokio::test]
    async fn setup_finishing_during_teardown_neither_fails_nor_revives_it() {
        let (state, recording, _dir, id) = provisioning_workspace().await;
        let thread_id = state.store.own_thread(&id).unwrap().unwrap().id;
        state
            .workspaces
            .set_status(&id, WorkspaceLifecycle::TearingDown)
            .await
            .unwrap();

        let finished = state
            .workspaces
            .finish_provisioning(
                &id,
                &thread_id,
                ThreadStatus::Idle,
                WorkspaceLifecycle::Ready,
                WorkspaceActivity::None,
            )
            .await;
        mark_failed(&state, &id, None, None, "late").await;

        assert!(finished.is_err());
        assert_eq!(
            state.workspaces.get(&id).await.unwrap().status,
            WorkspaceLifecycle::TearingDown
        );
        assert!(recording.toasts().is_empty());
    }
}
