//! Starting a workspace: naming it, recording it, and provisioning its worktree and Lead.

use std::path::PathBuf;

use uuid::Uuid;

use crate::composer::persist_user_message;
use crate::error::{Error, Result};
use crate::feedback::Output;
use crate::ipc::types::{CreateWorkspaceInput, StateChangeKind};
use crate::paths::{expand_tilde, worktrees_base};
use crate::state::AppState;
use crate::store::types::{ThreadRow, WorkspaceRow};
use crate::timeline::{TimelineStep, record_step};
use crate::workspace::ThreadStatus;

pub struct StartedWorkspace {
    pub workspace_id: String,
    pub thread_id: String,
    pub base: String,
}

/// A new worktree and its thread, provisioned in the background. `parent_thread_id` is the
/// thread that asked for it, when an agent did.
pub async fn start_workspace(
    out: &Output,
    state: &AppState,
    input: CreateWorkspaceInput,
    parent_thread_id: Option<String>,
) -> Result<StartedWorkspace> {
    let repo = state
        .store
        .repo_by_id(&input.repo_id)?
        .ok_or_else(|| Error::Git(format!("unknown repo {}", input.repo_id)))?;
    repo.ensure_not_default_branch(&input.branch)?;
    // Before any git call: `base` can come from an agent's `create_thread` call.
    crate::git::validate_ref_name(&input.branch, "branch")?;
    crate::git::validate_ref_name(&input.base, "base")?;
    let repo_path = expand_tilde(&repo.path);
    if !repo_path.is_dir() {
        return Err(Error::Git(format!(
            "repo path does not exist: {}",
            repo_path.display()
        )));
    }

    // With no name given, the prompt drafts one and Haiku improves it below, along with the
    // drafted branch. A workspace started without a prompt is numbered.
    let goal_text = input.goal.trim();
    let auto_named = input.name.trim().is_empty() && !goal_text.is_empty();
    let name = match input.name.trim() {
        "" if goal_text.is_empty() => {
            let live = state.store.live_workspaces()?;
            crate::naming::next_workspace_name(live.iter().map(|row| row.name.as_str()))
        }
        "" => crate::naming::draft_title(goal_text, crate::naming::WORKSPACE_NAME_MAX),
        given => given
            .chars()
            .take(crate::naming::WORKSPACE_NAME_MAX)
            .collect(),
    };
    let input = CreateWorkspaceInput { name, ..input };

    // Read before anything is recorded, so an attachment that can't be read starts nothing.
    let goal = input.goal.trim().to_string();
    let (prompt, shown) = if input.attachments.is_empty() {
        (None, goal.clone())
    } else {
        let folder = state.data_dir.join("attachments");
        let (text, attachments) = (goal.clone(), input.attachments.clone());
        let (prompt, shown) = tokio::task::spawn_blocking(move || {
            crate::attachments::build_prompt(&folder, &text, &attachments)
        })
        .await
        .map_err(|error| Error::Workspace(error.to_string()))??;
        (Some(prompt), shown)
    };

    let workspace_id = new_workspace_id();
    let thread_id = Uuid::new_v4().to_string();
    let repo_name = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    let worktrees_base = worktrees_base(&state.store)?;
    let worktree_path = crate::workspace::WorkspaceManager::worktree_path(
        &worktrees_base,
        repo_name,
        &input.branch,
    );

    state
        .workspace
        .register_provisioning(
            &workspace_id,
            &input.repo_id,
            &repo_path,
            &input.name,
            &input.branch,
            &input.base,
            &worktrees_base,
        )
        .await?;

    state.store.upsert_workspace(&WorkspaceRow {
        id: workspace_id.clone(),
        repo_id: input.repo_id.clone(),
        name: input.name.clone(),
        branch: input.branch.clone(),
        worktree_path: worktree_path.to_string_lossy().to_string(),
        status: "provisioning".into(),
        created_at: String::new(),
        summary: None,
        summary_at: None,
        summary_source: "Haiku 4.5".into(),
        kind: None,
        pr_number: None,
        pr_html_url: None,
        modified_files: 0,
        archived_at: None,
    })?;
    state.store.upsert_thread(&ThreadRow {
        id: thread_id.clone(),
        workspace_id: workspace_id.clone(),
        title: "Lead".into(),
        engine: input.engine.clone(),
        session_id: None,
        status: ThreadStatus::Provisioning.into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
        retired: false,
        parent_thread_id,
    })?;
    // Stored before provisioning starts, since the engine reads it when it spawns.
    if let Some(model) = input.model.as_deref() {
        state.store.set_thread_model(&thread_id, Some(model))?;
    }
    if let Some(effort) = input.effort.as_deref() {
        state.store.set_thread_effort(&thread_id, Some(effort))?;
    }
    if !shown.is_empty() {
        persist_user_message(&state.store, &thread_id, &shown)?;
    }

    state.notify(out, StateChangeKind::WorkspaceStatus);

    if auto_named {
        spawn_auto_name(
            out.clone(),
            state.clone(),
            workspace_id.clone(),
            input.name.clone(),
            goal.clone(),
        );
    }

    let (out_bg, state_bg) = (out.clone(), state.clone());
    let workspace_id_bg = workspace_id.clone();
    let thread_id_bg = thread_id.clone();
    let engine = input.engine.clone();
    let repo_name = repo.name.clone();
    let started = StartedWorkspace {
        workspace_id: workspace_id.clone(),
        thread_id: thread_id.clone(),
        base: input.base.clone(),
    };
    let setup_commands = crate::harness_config::effective_setup(&repo.setup_commands, &repo_path);

    let draft_branch = input.branch.clone();
    let base = input.base.clone();

    tauri::async_runtime::spawn(async move {
        let (out, state) = (out_bg, state_bg);
        if auto_named && let Err(error) = name_branch(&out, &state, &workspace_id_bg, &goal).await {
            log::warn!("keeping the drafted branch {draft_branch}: {error}");
        }
        let branch = match state.workspace.get(&workspace_id_bg).await {
            Some(record) => record.branch,
            None => draft_branch,
        };
        let worktree_step = TimelineStep::WorktreeCreated {
            branch,
            base: base.clone(),
        };
        if let Err(error) = record_step(&state.store, &thread_id_bg, worktree_step) {
            log::warn!("could not record the worktree step: {error}");
        }
        crate::provisioning::run_workspace_provisioning(
            out,
            state,
            crate::provisioning::LeadProvisionJob {
                workspace_id: workspace_id_bg,
                thread_id: thread_id_bg,
                repo_name,
                setup_commands_raw: setup_commands,
                engine,
                goal,
                prompt,
                review: false,
            },
        )
        .await;
    });

    Ok(started)
}

/// Swaps the drafted branch for Haiku's while nothing on disk uses it yet. The draft stands if
/// Haiku is slow or has nothing usable, since the agent shouldn't wait long for a name.
async fn name_branch(
    out: &Output,
    state: &AppState,
    workspace_id: &str,
    prompt: &str,
) -> Result<()> {
    let generated = tokio::time::timeout(
        crate::naming::BRANCH_NAME_TIMEOUT,
        crate::naming::generate_branch(&state.llm, prompt),
    )
    .await;
    let Ok(Some(branch)) = generated else {
        return Ok(());
    };
    let Some(record) = state.workspace.get(workspace_id).await else {
        return Ok(());
    };
    let repo_path = PathBuf::from(&record.repo_path);
    let (local, remote) = tokio::try_join!(
        state.git.list_local_branches(&repo_path),
        state.git.list_remote_branches(&repo_path, "origin"),
    )?;
    let mut taken = local;
    taken.extend(remote);
    taken.extend(
        state
            .store
            .live_workspaces()?
            .into_iter()
            .filter(|row| row.repo_id == record.repo_id && row.id != workspace_id)
            .map(|row| row.branch),
    );
    let branch = crate::naming::unique_branch(&branch, &taken);
    let record = state
        .workspace
        .retarget_provisioning(workspace_id, &branch)
        .await?;
    state
        .store
        .set_workspace_checkout(workspace_id, &record.branch, &record.worktree_path)?;
    state.notify(out, StateChangeKind::WorkspaceStatus);
    Ok(())
}

/// Swaps the drafted name for Haiku's unless the workspace was renamed or deleted meanwhile.
fn spawn_auto_name(
    out: Output,
    state: AppState,
    workspace_id: String,
    draft: String,
    prompt: String,
) {
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
        match state.store.workspace_by_id(&workspace_id) {
            Ok(Some(row)) if row.name == draft => {}
            _ => return,
        }
        if state
            .store
            .set_workspace_name(&workspace_id, &name)
            .is_err()
            || state.workspace.rename(&workspace_id, &name).await.is_err()
        {
            return;
        }
        state.notify(&out, StateChangeKind::WorkspaceStatus);
    });
}

fn new_workspace_id() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("ws{ms:x}")
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::*;
    use crate::feedback::{Emitted, RecordingOutput};
    use crate::shell_env::ShellEnv;
    use crate::store::types::RepoRecord;

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
        out: Output,
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

        let state = AppState::new(dir.path().join("data"));
        state.store.open_in_memory().unwrap();
        state
            .store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: repo.to_string_lossy().to_string(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        let worktrees = dir.path().join("worktrees");
        state
            .store
            .set_setting("worktreeRoot", &worktrees.to_string_lossy())
            .unwrap();
        set_path(&state, "/usr/bin:/bin").await;

        let (recording, out) = RecordingOutput::new();
        Fixture {
            state,
            recording,
            out,
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
            .upsert_workspace(&WorkspaceRow {
                id: "earlier".into(),
                repo_id: "r1".into(),
                name: "Workspace 3".into(),
                branch: "feat/earlier".into(),
                worktree_path: "/tmp/earlier".into(),
                status: "ready".into(),
                created_at: String::new(),
                summary: None,
                summary_at: None,
                summary_source: "Haiku 4.5".into(),
                kind: None,
                pr_number: None,
                pr_html_url: None,
                modified_files: 0,
                archived_at: None,
            })
            .unwrap();

        let started = start_workspace(&f.out, &f.state, input("", "feat/x", ""), None)
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
                .is_some_and(|thread| thread.has_status(ThreadStatus::Idle))
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
                .store
                .first_message_text(&started.thread_id)
                .unwrap(),
            None
        );
        assert!(matches!(
            f.recording.emitted().first(),
            Some(Emitted::State(StateChangeKind::WorkspaceStatus))
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

        let started = start_workspace(&f.out, &f.state, request, Some("parent".into()))
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
            store
                .first_message_text(&started.thread_id)
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

        let started = start_workspace(&f.out, &f.state, request, None)
            .await
            .unwrap();

        assert_eq!(
            f.state
                .store
                .first_message_text(&started.thread_id)
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
        let error = start_workspace(&f.out, &f.state, missing, None)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("gone.txt"), "{error}");
        assert_eq!(f.state.store.live_workspaces().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn the_draft_name_and_branch_stand_when_haiku_is_unavailable() {
        let f = fixture().await;

        let started = start_workspace(
            &f.out,
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
            &f.out,
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
        let record = f.state.workspace.get(&started.workspace_id).await.unwrap();
        assert_eq!(record.name, "OAuth login");
        assert_eq!(record.branch, "feat/oauth-login-2");
    }

    #[tokio::test]
    async fn a_name_given_by_hand_stands() {
        let f = fixture().await;
        install_fake_claude(&f).await;
        let draft = "Workspace named by the user";

        let started = start_workspace(&f.out, &f.state, input(draft, "feat/x", "Do it"), None)
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
            let error = start_workspace(&f.out, &f.state, request, None)
                .await
                .err()
                .unwrap();
            assert!(error.to_string().contains(message), "{error}");
        }

        std::fs::remove_dir_all(&f.repo).unwrap();
        let error = start_workspace(&f.out, &f.state, input("Work", "feat/x", ""), None)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("repo path does not exist"));

        assert!(f.state.store.live_workspaces().unwrap().is_empty());
        assert!(f.recording.emitted().is_empty());
    }
}
