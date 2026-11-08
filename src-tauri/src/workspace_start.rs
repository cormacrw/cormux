//! Starting a workspace: naming it, recording it, and provisioning its worktree and Lead.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};
use crate::model::WorkspaceKind;
use crate::provisioning::{ProvisionPlan, WorkspaceSpec};
use crate::state::AppState;
use crate::store::Store;
use crate::store::types::ThreadPicks;
use crate::timeline::TimelineStep;
use crate::workspace_git::Checkout;
use crate::workspaces::WorkspaceRecord;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub repo_id: String,
    pub name: String,
    /// Empty to have one drafted from the goal.
    pub branch: String,
    pub base: String,
    pub engine: String,
    pub goal: String,
    /// The model to start on; `None` leaves the engine's default.
    pub model: Option<String>,
    /// The effort level to start on; `None` leaves the engine's default.
    pub effort: Option<String>,
    /// Images and files sent with the goal, as the first message's attachments.
    #[serde(default)]
    pub attachments: Vec<crate::attachments::PromptAttachment>,
}

pub struct StartedWorkspace {
    pub workspace_id: String,
    pub thread_id: String,
    pub base: String,
}

/// A new worktree and its Lead, provisioned in the background. `parent_thread_id` is the
/// thread that asked for it, when an agent did.
pub async fn start_workspace(
    state: &AppState,
    input: CreateWorkspaceInput,
    parent_thread_id: Option<String>,
) -> Result<StartedWorkspace> {
    // With no name given, the prompt drafts one and Haiku improves it below, along with the
    // drafted branch. A workspace started without a prompt is numbered.
    let goal = input.goal.trim().to_string();
    let auto_named = input.name.trim().is_empty() && !goal.is_empty();
    let name: String = match input.name.trim() {
        "" if goal.is_empty() => {
            let live = state.store.live_workspaces()?;
            crate::naming::next_workspace_name(live.iter().map(|row| row.name.as_str()))
        }
        "" => crate::naming::draft_title(&goal, crate::naming::WORKSPACE_NAME_MAX),
        given => given
            .chars()
            .take(crate::naming::WORKSPACE_NAME_MAX)
            .collect(),
    };

    // Read before anything is recorded, so an attachment that can't be read starts nothing.
    let (prompt, shown) = if input.attachments.is_empty() {
        (None, goal.clone())
    } else {
        let folder = state.data_dir.attachments();
        let (text, attachments) = (goal.clone(), input.attachments.clone());
        let (prompt, shown) = tokio::task::spawn_blocking(move || {
            crate::attachments::build_prompt(&folder, &text, &attachments)
        })
        .await
        .map_err(|error| Error::Workspace(error.to_string()))??;
        (Some(prompt), shown)
    };

    let branch = match input.branch.trim() {
        "" => {
            let repo = state.store.repo(&input.repo_id)?;
            let repo_path = crate::paths::expand_tilde(&repo.path);
            let taken = taken_branches(state, &repo.id, &repo_path, None).await?;
            crate::naming::draft_branch(&goal, &taken)
        }
        given => given.to_string(),
    };

    let created = crate::provisioning::create(
        state,
        WorkspaceSpec {
            kind: WorkspaceKind::Lead,
            repo_id: input.repo_id.clone(),
            name: name.clone(),
            branch: branch.clone(),
            base: input.base.clone(),
            pr: None,
            engine: input.engine.clone(),
            parent_thread_id,
            picks: ThreadPicks {
                model: input.model.clone(),
                effort: input.effort.clone(),
            },
        },
    )
    .await?;
    let workspace_id = created.workspace.id.clone();
    let thread_id = created.thread_id.clone();
    if !shown.is_empty()
        && let Err(error) = state.transcript.record_user_message(&thread_id, &shown)
    {
        // Recorded already: failing its setup leaves it to retry or tear down.
        crate::provisioning::fail(state, &workspace_id, &error.to_string()).await;
        return Err(error);
    }

    state.out.changed();

    if auto_named {
        spawn_auto_name(state.clone(), workspace_id.clone(), name, goal.clone());
    }

    let plan = lead_plan(&workspace_id, &thread_id, &goal, prompt);
    let state_bg = state.clone();
    let draft_branch = branch;
    tauri::async_runtime::spawn(async move {
        let state = state_bg;
        if auto_named && let Err(error) = name_branch(&state, &plan.workspace_id, &goal).await {
            log::warn!("keeping the drafted branch {draft_branch}: {error}");
        }
        if let Ok(workspace) = state.workspaces.get(&plan.workspace_id).await {
            let step = TimelineStep::WorktreeCreated {
                branch: workspace.branch,
                base: workspace.base,
            };
            if let Err(error) = state.transcript.record_step(&plan.thread_id, step) {
                log::warn!("could not record the worktree step: {error}");
            }
        }
        crate::provisioning::run(&state, plan).await;
    });

    Ok(StartedWorkspace {
        workspace_id,
        thread_id,
        base: input.base,
    })
}

/// Told to a Lead whose branch was cut from the repo's default branch: the user may have meant to
/// start from another branch, and only the first message can say so.
const BRANCH_INSTRUCTION: &str = "This thread starts on a new branch cut from the repo's default branch, with a worktree of its own. If the user's first message names another branch to start from, or asks to pick up earlier work, run `git reset --hard <branch>` (`origin/<branch>` if it only exists there) before anything else, while this branch has no commits of its own, and say which branch you started from. Ignore this for later messages, and don't do it for a branch the user only mentions.";

/// What an agent is told about its workspace when a session starts. The branch instruction only
/// holds for a Lead's first session on a branch cut from the repo's default branch: a Reviewer
/// sits on a pull request's branch, and a workspace started from another base was started there
/// on purpose.
pub fn session_brief(store: &Store, workspace: &WorkspaceRecord, resuming: bool) -> String {
    let media = crate::media::instruction();
    if resuming || workspace.kind != WorkspaceKind::Lead {
        return media;
    }
    let default_branch = match store.repo_by_id(&workspace.repo_id) {
        Ok(Some(repo)) => repo.default_branch,
        Ok(None) => return media,
        Err(error) => {
            log::warn!("reading {}'s repo for its brief: {error}", workspace.id);
            return media;
        }
    };
    if workspace.base != default_branch {
        return media;
    }
    format!("{BRANCH_INSTRUCTION}\n\n{media}")
}

/// `prompt` is the goal with its attachments, when there were any.
fn lead_plan(
    workspace_id: &str,
    thread_id: &str,
    goal: &str,
    prompt: Option<Prompt>,
) -> ProvisionPlan {
    let goal = goal.trim();
    let first_prompt = prompt.or_else(|| (!goal.is_empty()).then(|| goal.into()));
    ProvisionPlan {
        workspace_id: workspace_id.to_string(),
        thread_id: thread_id.to_string(),
        checkout: Checkout::NewBranch,
        first_prompt,
        intro: Some(Box::new(|workspace: &WorkspaceRecord| {
            format!(
                "Started “{}” on a branch from {}.",
                workspace.name, workspace.base
            )
        })),
        started_toast: None,
    }
}

/// The plan again for a Lead whose setup failed. Attachments sent with the goal are lost: the
/// goal goes as the thread shows it.
pub fn resume_plan(state: &AppState, workspace: &WorkspaceRecord) -> Result<ProvisionPlan> {
    let thread = state
        .store
        .own_thread(&workspace.id)?
        .ok_or_else(|| Error::Workspace(format!("no Lead in workspace {}", workspace.id)))?;
    let goal = state
        .transcript
        .first_message(&thread.id)?
        .unwrap_or_default();
    Ok(lead_plan(&workspace.id, &thread.id, &goal, None))
}

/// Swaps the drafted branch for Haiku's while nothing on disk uses it yet. The draft stands if
/// Haiku is slow or has nothing usable, since the agent shouldn't wait long for a name.
async fn name_branch(state: &AppState, workspace_id: &str, prompt: &str) -> Result<()> {
    let generated = tokio::time::timeout(
        crate::naming::BRANCH_NAME_TIMEOUT,
        crate::naming::generate_branch(&state.llm, prompt),
    )
    .await;
    let Ok(Some(branch)) = generated else {
        return Ok(());
    };
    let workspace = state.workspaces.get(workspace_id).await?;
    let taken = taken_branches(
        state,
        &workspace.repo_id,
        Path::new(&workspace.repo_path),
        Some(workspace_id),
    )
    .await?;
    let branch = crate::naming::unique_branch(&branch, &taken);
    crate::git::validate_ref_name(&branch, "branch")?;
    state.workspaces.retarget(workspace_id, &branch).await?;
    state.out.changed();
    Ok(())
}

async fn taken_branches(
    state: &AppState,
    repo_id: &str,
    repo_path: &Path,
    except_workspace: Option<&str>,
) -> Result<Vec<String>> {
    let (local, remote) = tokio::try_join!(
        state.git.list_local_branches(repo_path),
        state.git.list_remote_branches(repo_path, "origin"),
    )?;
    let mut taken = local;
    taken.extend(remote);
    taken.extend(
        state
            .store
            .live_workspaces()?
            .into_iter()
            .filter(|row| row.repo_id == repo_id && Some(row.id.as_str()) != except_workspace)
            .map(|row| row.branch),
    );
    Ok(taken)
}

/// Swaps the drafted name for Haiku's unless the workspace was renamed or deleted meanwhile.
fn spawn_auto_name(state: AppState, workspace_id: String, draft: String, prompt: String) {
    tauri::async_runtime::spawn(async move {
        let Some(name) = crate::naming::generate_title(
            &state.llm,
            &prompt,
            "task",
            crate::naming::WORKSPACE_NAME_MAX,
        )
        .await
        else {
            return;
        };
        match state
            .workspaces
            .rename_from(&workspace_id, &draft, &name)
            .await
        {
            Ok(true) => state.out.changed(),
            Ok(false) => {}
            Err(error) => log::warn!("naming workspace {workspace_id}: {error}"),
        }
    });
}

pub mod commands {
    use serde::{Deserialize, Serialize};
    use specta::Type;
    use tauri::State;

    use crate::error::Result;
    use crate::model::WorkspaceKind;
    use crate::provisioning::ProvisionPlan;
    use crate::state::AppState;

    use super::CreateWorkspaceInput;

    #[derive(Debug, Clone, Serialize, Deserialize, Type)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateWorkspaceResult {
        pub workspace_id: String,
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn create_workspace(
        input: CreateWorkspaceInput,
        state: State<'_, AppState>,
    ) -> Result<CreateWorkspaceResult> {
        let started = super::start_workspace(&state, input, None).await?;
        Ok(CreateWorkspaceResult {
            workspace_id: started.workspace_id,
        })
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn retry_workspace_provisioning(
        workspace_id: String,
        state: State<'_, AppState>,
    ) -> Result<()> {
        let plan = resume_plan(&state, &workspace_id).await?;
        crate::provisioning::retry(&state, plan).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn skip_workspace_provisioning_setup(
        workspace_id: String,
        state: State<'_, AppState>,
    ) -> Result<()> {
        let plan = resume_plan(&state, &workspace_id).await?;
        crate::provisioning::skip_setup(&state, plan).await
    }

    async fn resume_plan(state: &AppState, workspace_id: &str) -> Result<ProvisionPlan> {
        let workspace = state.workspaces.get(workspace_id).await?;
        match workspace.kind {
            WorkspaceKind::Lead => super::resume_plan(state, &workspace),
            WorkspaceKind::PrReview => crate::review::resume_plan(state, &workspace),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::*;
    use crate::feedback::{Emitted, RecordingOutput};
    use crate::model::{ThreadStatus, WorkspaceLifecycle};
    use crate::shell_env::ShellEnv;
    use crate::store::types::{RepoRecord, WorkspaceRow};

    /// Stands in for the `claude` CLI: answers one-shot naming calls and refuses to be an engine.
    const FAKE_CLAUDE: &str = r#"#!/bin/sh
case "$*" in
  *"--output-format json"*)
    if grep -q "git branch name"; then
      echo '{"result":"feat/oauth-login"}'
    else
      echo '{"result":"OAuth login"}'
    fi
    ;;
  *) exit 1 ;;
esac
"#;

    struct Fixture {
        state: AppState,
        recording: Arc<RecordingOutput>,
        repo: PathBuf,
        worktrees: PathBuf,
        dir: TempDir,
    }

    fn git(repo: &Path, args: &[&str]) {
        let result = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(result.status.success(), "git {args:?}: {result:?}");
    }

    /// A registered repo on `main` with one commit, and worktrees kept in the temp folder. The
    /// shell env is pinned, so no test can reach a real `claude` or `gh` on this machine.
    async fn fixture() -> Fixture {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("app");
        std::fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "cormux@test"]);
        git(&repo, &["config", "user.name", "Cormux"]);
        git(&repo, &["commit", "-q", "--allow-empty", "-m", "init"]);

        let (state, recording) =
            AppState::recorded(crate::paths::DataDir::new(dir.path().join("data")));
        state
            .store
            .upsert_repo(&RepoRecord {
                path: repo.to_string_lossy().to_string(),
                name: "app".into(),
                ..crate::store::fixtures::repo("r1")
            })
            .unwrap();
        let worktrees = dir.path().join("worktrees");
        crate::settings::Settings::update(
            &state.store,
            crate::settings::SettingsPatch {
                worktree_root: Some(worktrees.to_string_lossy().into()),
                ..Default::default()
            },
        )
        .unwrap();
        set_path(&state, "/usr/bin:/bin").await;

        Fixture {
            state,
            recording,
            repo,
            worktrees,
            dir,
        }
    }

    async fn set_path(state: &AppState, path: &str) {
        *state.shell_env.write().await =
            ShellEnv::from_vars([("PATH".to_string(), path.to_string())]);
    }

    async fn install_fake_claude(fixture: &Fixture) {
        use std::os::unix::fs::PermissionsExt;

        let bin = fixture.dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let script = bin.join("claude");
        std::fs::write(&script, FAKE_CLAUDE).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        set_path(&fixture.state, &format!("{}:/usr/bin:/bin", bin.display())).await;
    }

    fn input(name: &str, branch: &str, goal: &str) -> CreateWorkspaceInput {
        CreateWorkspaceInput {
            repo_id: "r1".into(),
            name: name.into(),
            branch: branch.into(),
            base: "main".into(),
            engine: "claude".into(),
            goal: goal.into(),
            model: None,
            effort: None,
            attachments: Vec::new(),
        }
    }

    /// Provisioning runs on Tauri's runtime, so its results can only be waited for.
    async fn eventually(what: &str, mut done: impl FnMut() -> bool) {
        for _ in 0..1000 {
            if done() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("timed out waiting for {what}");
    }

    fn workspace(state: &AppState, id: &str) -> WorkspaceRow {
        state.store.workspace_by_id(id).unwrap().unwrap()
    }

    fn step_payloads(state: &AppState, thread_id: &str) -> Vec<String> {
        state
            .store
            .thread_event_page(thread_id, None, None, None)
            .unwrap()
            .events
            .into_iter()
            .filter(|event| event.kind == "tool")
            .map(|event| event.payload)
            .collect()
    }

    fn has_step(state: &AppState, thread_id: &str, text: &str) -> bool {
        step_payloads(state, thread_id)
            .iter()
            .any(|payload| payload.contains(text))
    }

    #[tokio::test]
    async fn with_no_prompt_a_numbered_workspace_gets_a_real_worktree_and_an_idle_lead() {
        let f = fixture().await;
        f.state
            .store
            .insert_workspace(
                &WorkspaceRow::new(
                    WorkspaceKind::Lead,
                    "earlier",
                    "r1",
                    "Workspace 3",
                    "feat/earlier",
                    "main",
                    "/tmp/earlier",
                )
                .with_status(WorkspaceLifecycle::Ready),
            )
            .unwrap();

        let started = start_workspace(&f.state, input("", "feat/x", ""), None)
            .await
            .unwrap();

        assert_eq!(started.base, "main");
        assert!(started.workspace_id.starts_with("ws"));
        let thread_id = started.thread_id.clone();
        let state = f.state.clone();
        eventually("the Lead to go idle", move || {
            state
                .store
                .thread_by_id(&thread_id)
                .unwrap()
                .is_some_and(|thread| thread.status == ThreadStatus::Idle)
        })
        .await;

        let row = workspace(&f.state, &started.workspace_id);
        assert_eq!(row.name, "Workspace 4");
        assert_eq!(row.branch, "feat/x");
        let worktree = f.worktrees.join("app").join("feat-x");
        assert_eq!(row.worktree_path, worktree.to_string_lossy());
        assert_eq!(
            f.state.git.current_branch(&worktree).await.unwrap(),
            "feat/x"
        );

        let thread = f
            .state
            .store
            .threads_for_workspace(&started.workspace_id)
            .unwrap();
        assert_eq!(thread.len(), 1);
        assert_eq!(thread[0].title, "Lead");
        assert!(has_step(&f.state, &started.thread_id, "feat/x ← main"));
        assert!(has_step(&f.state, &started.thread_id, "Worktree ready"));
        assert_eq!(
            f.state
                .transcript
                .first_message(&started.thread_id)
                .unwrap(),
            None
        );
        assert!(matches!(
            f.recording.emitted().first(),
            Some(Emitted::State(_))
        ));
    }

    #[tokio::test]
    async fn what_the_engine_reads_is_stored_before_start_workspace_returns() {
        let f = fixture().await;
        let long_name = "n".repeat(crate::naming::WORKSPACE_NAME_MAX + 10);
        let request = CreateWorkspaceInput {
            model: Some("opus".into()),
            effort: Some("high".into()),
            ..input(&long_name, "feat/x", "  Fix the login page  ")
        };

        let started = start_workspace(&f.state, request, Some("parent".into()))
            .await
            .unwrap();

        let store = &f.state.store;
        let row = workspace(&f.state, &started.workspace_id);
        assert_eq!(row.name, "n".repeat(crate::naming::WORKSPACE_NAME_MAX));
        assert_eq!(
            store.thread_model(&started.thread_id).unwrap().as_deref(),
            Some("opus")
        );
        assert_eq!(
            store.thread_effort(&started.thread_id).unwrap().as_deref(),
            Some("high")
        );
        assert_eq!(
            f.state
                .transcript
                .first_message(&started.thread_id)
                .unwrap()
                .as_deref(),
            Some("Fix the login page")
        );
        let thread = store.thread_by_id(&started.thread_id).unwrap().unwrap();
        assert_eq!(thread.parent_thread_id.as_deref(), Some("parent"));
    }

    #[tokio::test]
    async fn attachments_join_the_first_message_and_an_unreadable_one_starts_nothing() {
        use crate::attachments::PromptAttachment;

        let f = fixture().await;
        let pasted = PromptAttachment {
            name: "shot.png".into(),
            path: None,
            data_base64: Some("aGk=".into()),
        };
        let request = CreateWorkspaceInput {
            attachments: vec![pasted],
            ..input("", "feat/x", "Fix this")
        };

        let started = start_workspace(&f.state, request, None).await.unwrap();

        assert_eq!(
            f.state
                .transcript
                .first_message(&started.thread_id)
                .unwrap()
                .as_deref(),
            Some("Fix this\n\nAttached: shot.png")
        );
        assert_eq!(workspace(&f.state, &started.workspace_id).name, "Fix this");

        let missing = CreateWorkspaceInput {
            attachments: vec![PromptAttachment {
                name: "gone.txt".into(),
                path: Some("/nonexistent/gone.txt".into()),
                data_base64: None,
            }],
            ..input("", "feat/y", "Look")
        };
        let error = start_workspace(&f.state, missing, None)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("gone.txt"), "{error}");
        assert_eq!(f.state.store.live_workspaces().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn without_a_branch_one_is_drafted_from_the_goal_past_taken_names() {
        let f = fixture().await;
        git(&f.repo, &["branch", "fix/fix-the-login-page"]);

        let started = start_workspace(&f.state, input("", "", "Fix the login page."), None)
            .await
            .unwrap();

        assert_eq!(
            workspace(&f.state, &started.workspace_id).branch,
            "fix/fix-the-login-page-2"
        );
    }

    #[tokio::test]
    async fn the_draft_name_and_branch_stand_when_haiku_is_unavailable() {
        let f = fixture().await;

        let started = start_workspace(
            &f.state,
            input("", "feat/draft", "Fix the login page."),
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            workspace(&f.state, &started.workspace_id).name,
            "Fix the login page"
        );
        let (state, thread_id) = (f.state.clone(), started.thread_id.clone());
        eventually("the worktree step", move || {
            has_step(&state, &thread_id, "Created worktree")
        })
        .await;
        assert!(has_step(&f.state, &started.thread_id, "feat/draft ← main"));
        assert_eq!(
            workspace(&f.state, &started.workspace_id).name,
            "Fix the login page"
        );
    }

    #[tokio::test]
    async fn haikus_name_and_an_unused_branch_replace_the_draft() {
        let f = fixture().await;
        install_fake_claude(&f).await;
        git(&f.repo, &["branch", "feat/oauth-login"]);

        let started = start_workspace(
            &f.state,
            input("", "feat/draft", "Let people sign in with OAuth"),
            None,
        )
        .await
        .unwrap();

        let (state, id) = (f.state.clone(), started.workspace_id.clone());
        eventually("Haiku's name and branch", move || {
            let row = workspace(&state, &id);
            row.name == "OAuth login" && row.branch == "feat/oauth-login-2"
        })
        .await;
        let (state, thread_id) = (f.state.clone(), started.thread_id.clone());
        eventually("the worktree step", move || {
            has_step(&state, &thread_id, "feat/oauth-login-2 ← main")
        })
        .await;

        let worktree = f.worktrees.join("app").join("feat-oauth-login-2");
        assert_eq!(
            workspace(&f.state, &started.workspace_id).worktree_path,
            worktree.to_string_lossy()
        );
        let record = f.state.workspaces.get(&started.workspace_id).await.unwrap();
        assert_eq!(record.name, "OAuth login");
        assert_eq!(record.branch, "feat/oauth-login-2");
    }

    #[tokio::test]
    async fn a_name_given_by_hand_stands() {
        let f = fixture().await;
        install_fake_claude(&f).await;
        let draft = "Workspace named by the user";

        let started = start_workspace(&f.state, input(draft, "feat/x", "Do it"), None)
            .await
            .unwrap();

        let (state, thread_id) = (f.state.clone(), started.thread_id.clone());
        eventually("the worktree step", move || {
            has_step(&state, &thread_id, "feat/x ← main")
        })
        .await;
        assert_eq!(workspace(&f.state, &started.workspace_id).name, draft);
    }

    #[tokio::test]
    async fn nothing_is_recorded_for_a_request_that_cant_start() {
        let f = fixture().await;

        let unknown = CreateWorkspaceInput {
            repo_id: "nope".into(),
            ..input("Work", "feat/x", "")
        };
        let cases = [
            (unknown, "unknown repo nope"),
            (input("Work", " main ", ""), "the default branch of app"),
        ];
        for (request, message) in cases {
            let error = start_workspace(&f.state, request, None)
                .await
                .err()
                .unwrap();
            assert!(error.to_string().contains(message), "{error}");
        }

        std::fs::remove_dir_all(&f.repo).unwrap();
        let error = start_workspace(&f.state, input("Work", "feat/x", ""), None)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("repo path does not exist"));

        assert!(f.state.store.live_workspaces().unwrap().is_empty());
        assert!(f.recording.emitted().is_empty());
    }

    #[tokio::test]
    async fn only_a_leads_first_session_off_the_default_branch_is_told_it_may_reset() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Ready);
        let lead = crate::workspaces::Workspaces::new(store.clone())
            .get("w1")
            .await
            .unwrap();
        let told = |workspace: &WorkspaceRecord, resuming| {
            session_brief(&store, workspace, resuming).contains("git reset --hard")
        };

        assert!(told(&lead, false));
        assert!(
            !told(&lead, true),
            "a resumed session already had its first message"
        );
        let reviewer = WorkspaceRecord {
            kind: WorkspaceKind::PrReview,
            ..lead.clone()
        };
        assert!(
            !told(&reviewer, false),
            "a Reviewer sits on the PR's branch"
        );
        let stacked = WorkspaceRecord {
            base: "feat/base".into(),
            ..lead.clone()
        };
        assert!(
            !told(&stacked, false),
            "started from another base on purpose"
        );
        for workspace in [&lead, &reviewer, &stacked] {
            assert!(session_brief(&store, workspace, true).contains(&crate::media::instruction()));
        }
    }
}
