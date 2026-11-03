//! Registered repos: adding and removing them, their settings, and git in their main checkouts.

use std::path::Path;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::feedback::Output;
use crate::ipc::types::{
    AddRepoInput, BranchDeletion, LocalBranchRow, LocalBranchesResult, RemoveRepoInput,
    RepoBranchesResult, RepoGitRuntime, SetRepoDefaultBranchInput, SetRepoRunCommandInput,
    SetRepoSetupCommandsInput, SetRepoSingleInstanceInput, StateChangeKind, TestRepoSetupInput,
    TestRepoSetupResult, ToastPart, ToastRaisedPayload, ToastTone,
};
use crate::paths::expand_tilde;
use crate::state::AppState;
use crate::store::types::RepoRecord;

fn repo_paths_match(left: &str, right: &Path) -> bool {
    let left_path = expand_tilde(left);
    left_path.canonicalize().ok() == right.canonicalize().ok()
}

fn upsert_repo_record(out: &Output, state: &AppState, repo: RepoRecord) -> Result<()> {
    state.store.upsert_repo(&repo)?;
    state.notify(out, StateChangeKind::WorkspaceStatus);
    Ok(())
}

/// A registered repo, or the error every repo command gives for an unknown id.
fn repo_record(state: &AppState, repo_id: &str) -> Result<RepoRecord> {
    state
        .store
        .repo_by_id(repo_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown repo {repo_id}")))
}

pub fn set_run_command(
    out: &Output,
    state: &AppState,
    input: SetRepoRunCommandInput,
) -> Result<()> {
    let mut repo = repo_record(state, &input.repo_id)?;
    repo.run_command = input
        .run_command
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    upsert_repo_record(out, state, repo)
}

pub fn set_single_instance(
    out: &Output,
    state: &AppState,
    input: SetRepoSingleInstanceInput,
) -> Result<()> {
    let mut repo = repo_record(state, &input.repo_id)?;
    repo.single_instance = input.single_instance;
    upsert_repo_record(out, state, repo)
}

pub fn set_setup_commands(
    out: &Output,
    state: &AppState,
    input: SetRepoSetupCommandsInput,
) -> Result<()> {
    let mut repo = repo_record(state, &input.repo_id)?;
    repo.setup_commands = input.setup_commands;
    upsert_repo_record(out, state, repo)
}

pub fn set_default_branch(
    out: &Output,
    state: &AppState,
    input: SetRepoDefaultBranchInput,
) -> Result<()> {
    let mut repo = repo_record(state, &input.repo_id)?;
    let branch = input.default_branch.trim().to_string();
    if branch.is_empty() {
        return Err(Error::Workspace("Pick a default branch".into()));
    }
    if let Some(workspace) = state
        .store
        .live_workspaces()?
        .iter()
        .find(|row| row.repo_id == repo.id && row.branch == branch)
    {
        return Err(Error::Workspace(format!(
            "{} has {branch} checked out. Switch it to another branch first.",
            workspace.name
        )));
    }
    repo.default_branch = Some(branch);
    upsert_repo_record(out, state, repo)
}

/// Commits each repo's default branch is behind and ahead of `origin`, against the last
/// fetch. A repo with no `origin` copy of the branch reads as 0 and 0.
pub async fn git_runtime(state: &AppState) -> Result<Vec<RepoGitRuntime>> {
    let mut rows = Vec::new();
    for repo in state.store.repos()? {
        let path = expand_tilde(&repo.path);
        let branch = repo.default_branch_or_main();
        // One `rev-list --left-right` gives both counts, instead of a process per direction.
        let (ahead, behind) = state
            .git
            .ahead_behind(&path, branch, &format!("origin/{branch}"))
            .await?;
        rows.push(RepoGitRuntime {
            repo_id: repo.id,
            behind,
            ahead,
        });
    }
    Ok(rows)
}

/// Fast-forward the repo checkout's default branch from origin.
pub async fn pull_default_branch(out: &Output, state: &AppState, repo_id: &str) -> Result<()> {
    let repo = repo_record(state, repo_id)?;
    let branch = repo.default_branch_or_main().to_string();
    let pulled = state
        .git
        .pull_branch(&expand_tilde(&repo.path), &branch)
        .await?;

    // Workspaces count how far behind origin/<base> they are, which the pull just moved.
    let updates = state.fetch.tick().await?;
    crate::git_workspace::apply_behind_updates(state, &updates).await;
    state.notify(out, StateChangeKind::BehindCounts);

    let summary = match pulled {
        0 => " is up to date".to_string(),
        1 => ": pulled 1 commit".to_string(),
        count => format!(": pulled {count} commits"),
    };
    out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![
            ToastPart::Text {
                value: format!("{} ", repo.name),
            },
            ToastPart::Code { value: branch },
            ToastPart::Text { value: summary },
        ],
        workspace_id: None,
        thread_id: None,
    });
    Ok(())
}

pub async fn add(out: &Output, state: &AppState, input: AddRepoInput) -> Result<RepoRecord> {
    let (normalized, id) = crate::harness_config::validate_add_path(&input.path)?;
    let expanded = expand_tilde(&normalized);
    if !expanded.is_dir() {
        return Err(Error::Git(format!(
            "folder does not exist: {}",
            expanded.display()
        )));
    }

    let toplevel = state
        .git
        .show_toplevel(&expanded)
        .await
        .map_err(|_| Error::Git("That folder is not a git repository".into()))?;
    let display_path = crate::harness_config::path_for_display(&toplevel);

    let repos = state.store.repos()?;
    if repos
        .iter()
        .any(|row| repo_paths_match(&row.path, &toplevel))
    {
        return Err(Error::Workspace("That folder is already added".into()));
    }
    if repos.iter().any(|row| row.id == id) {
        return Err(Error::Workspace(format!(
            "A repo named {id} is already added"
        )));
    }

    let default_branch = state.git.default_branch(&toplevel).await.ok();
    let committed = crate::harness_config::read_committed_config(&toplevel);
    let suggested = crate::harness_config::suggest_commands(&toplevel);
    let (setup_commands, run_command) =
        crate::harness_config::merge_initial_config(committed, suggested);

    let name = id.clone();
    let record = RepoRecord {
        id,
        path: display_path.clone(),
        name,
        default_branch,
        setup_commands,
        run_command,
        single_instance: false,
    };
    upsert_repo_record(out, state, record.clone())?;

    out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![ToastPart::Text {
            value: format!(
                "Added {}. Add its setup and run commands below.",
                record.name
            ),
        }],
        workspace_id: None,
        thread_id: None,
    });
    resync_prs(out, state);

    Ok(record)
}

pub async fn remove(out: &Output, state: &AppState, input: RemoveRepoInput) -> Result<()> {
    let repo = repo_record(state, &input.repo_id)?;

    let used = state.store.count_active_workspaces_for_repo(&repo.id)?;
    if used > 0 {
        let tear = if used == 1 { "it" } else { "them" };
        return Err(Error::Workspace(format!(
            "{} has {used} workspace{}. Tear {tear} down first.",
            repo.name,
            if used == 1 { "" } else { "s" }
        )));
    }
    if state.store.repo_count()? <= 1 {
        return Err(Error::Workspace("Harness needs at least one repo".into()));
    }

    state.store.delete_repo(&repo.id)?;
    state.notify(out, StateChangeKind::WorkspaceStatus);

    out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![ToastPart::Text {
            value: format!("Removed {}. The folder on disk wasn't touched.", repo.name),
        }],
        workspace_id: None,
        thread_id: None,
    });
    resync_prs(out, state);
    Ok(())
}

/// The PR list only shows registered repos, so re-sync when that set changes.
fn resync_prs(out: &Output, state: &AppState) {
    let (out, state) = (out.clone(), state.clone());
    tauri::async_runtime::spawn(async move {
        if let Err(error) = state.pr_sync.sync_app(&out, &state).await {
            log::warn!("PR sync after repo change failed: {error}");
        }
    });
}

pub async fn test_setup(
    state: &AppState,
    input: TestRepoSetupInput,
) -> Result<TestRepoSetupResult> {
    let repo = repo_record(state, &input.repo_id)?;
    let repo_path = expand_tilde(&repo.path);
    let setup_raw = crate::harness_config::effective_setup(&repo.setup_commands, &repo_path);
    let commands = crate::provisioning::parse_setup_commands(&setup_raw);
    if commands.is_empty() {
        return Ok(TestRepoSetupResult {
            ok: true,
            message: "No setup commands to run.".into(),
        });
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let harness_env = vec![
        ("HARNESS_REPO_PATH".into(), repo.path.clone()),
        (
            "HARNESS_BRANCH".into(),
            repo.default_branch.clone().unwrap_or_else(|| "main".into()),
        ),
    ];

    for (index, command) in commands.iter().enumerate() {
        let session_id = format!("test-setup-{}-{index}", repo.id);
        state.process.spawn_session(
            &session_id,
            &shell,
            &["-l", "-c", command],
            Some(&repo_path),
            &harness_env,
        )?;
        let deadline = std::time::Instant::now() + crate::provisioning::SETUP_COMMAND_TIMEOUT;
        loop {
            if let Some(code) = state.process.exit_code(&session_id) {
                if code != 0 {
                    return Ok(TestRepoSetupResult {
                        ok: false,
                        message: format!("Command failed ({code}): {command}"),
                    });
                }
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = state.process.stop_session(&session_id);
                return Ok(TestRepoSetupResult {
                    ok: false,
                    message: format!("Command timed out: {command}"),
                });
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    Ok(TestRepoSetupResult {
        ok: true,
        message: format!(
            "Ran {} setup command{} in the repo checkout.",
            commands.len(),
            if commands.len() == 1 { "" } else { "s" }
        ),
    })
}

pub async fn list_branches(state: &AppState, repo_id: &str) -> Result<RepoBranchesResult> {
    let repo = repo_record(state, repo_id)?;
    let repo_path = expand_tilde(&repo.path);
    // Branches that only exist on GitHub can be checked out too; `git switch` tracks them.
    let (local, remote) = tokio::try_join!(
        state.git.list_local_branches(&repo_path),
        state.git.list_remote_branches(&repo_path, "origin"),
    )?;
    let mut names = local;
    names.extend(remote);
    names.sort();
    names.dedup();
    Ok(RepoBranchesResult { branches: names })
}

pub async fn list_local_branches(state: &AppState, repo_id: &str) -> Result<LocalBranchesResult> {
    let repo = repo_record(state, repo_id)?;
    let repo_path = expand_tilde(&repo.path);
    let (head, tips) = tokio::join!(
        state.git.current_branch(&repo_path),
        state.git.list_local_branch_tips(&repo_path),
    );
    let head = head.unwrap_or_default();
    let branches = tips?
        .into_iter()
        .map(|tip| LocalBranchRow {
            name: tip.name,
            committed: tip.committed,
            subject: tip.subject,
        })
        .collect();
    Ok(LocalBranchesResult { head, branches })
}

pub async fn delete_local_branch(
    state: &AppState,
    repo_id: &str,
    branch: &str,
    force: bool,
) -> Result<BranchDeletion> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(Error::Git("missing branch name".into()));
    }
    let repo = repo_record(state, repo_id)?;
    if branch == repo.default_branch_or_main() {
        return Err(Error::Git(format!("{branch} is the default branch")));
    }
    if let Some(workspace) = state
        .store
        .live_workspaces()?
        .iter()
        .find(|row| row.repo_id == repo_id && row.branch == branch)
    {
        return Err(Error::Git(format!(
            "{branch} is checked out by {}",
            workspace.name
        )));
    }
    let repo_path = expand_tilde(&repo.path);
    let head = state
        .git
        .current_branch(&repo_path)
        .await
        .unwrap_or_default();
    if !head.is_empty() && head == branch {
        return Err(Error::Git(format!("{branch} is checked out in the repo")));
    }
    if !force && !state.git.in_sync_with_origin(&repo_path, branch).await? {
        return Ok(BranchDeletion::Unpushed);
    }
    state.git.branch_delete(&repo_path, branch).await?;
    Ok(BranchDeletion::Deleted)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use tempfile::TempDir;

    use super::*;
    use crate::feedback::RecordingOutput;
    use crate::store::types::WorkspaceRow;

    fn git(repo: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(status.status.success(), "git {args:?}: {status:?}");
    }

    /// A git repo with one commit on `main`, in a folder called `name`.
    fn init_repo(dir: &TempDir, name: &str) -> PathBuf {
        let repo = dir.path().join(name);
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "cormux@test"]);
        git(&repo, &["config", "user.name", "Cormux"]);
        git(&repo, &["commit", "-q", "--allow-empty", "-m", "init"]);
        repo
    }

    fn fixture() -> (AppState, Arc<RecordingOutput>, Output, TempDir) {
        let dir = TempDir::new().unwrap();
        let state = AppState::new(dir.path().join("data"));
        state.store.open_in_memory().unwrap();
        let (recording, out) = RecordingOutput::new();
        (state, recording, out, dir)
    }

    fn register(state: &AppState, id: &str, path: &Path) {
        state
            .store
            .upsert_repo(&RepoRecord {
                id: id.into(),
                path: path.to_string_lossy().to_string(),
                name: id.into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
    }

    fn live_workspace(state: &AppState, repo_id: &str, branch: &str) {
        state
            .store
            .upsert_workspace(&WorkspaceRow {
                id: "w1".into(),
                repo_id: repo_id.into(),
                name: "Work".into(),
                branch: branch.into(),
                worktree_path: "/tmp/wt".into(),
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
    }

    async fn add_path(out: &Output, state: &AppState, path: &Path) -> Result<RepoRecord> {
        let input = AddRepoInput {
            path: path.to_string_lossy().to_string(),
        };
        add(out, state, input).await
    }

    #[tokio::test]
    async fn adding_a_repo_records_it_then_refreshes_and_toasts() {
        let (state, recording, out, dir) = fixture();
        let repo = init_repo(&dir, "app");

        let record = add_path(&out, &state, &repo).await.unwrap();

        assert_eq!(record.id, "app");
        assert_eq!(record.name, "app");
        assert_eq!(
            expand_tilde(&record.path).canonicalize().unwrap(),
            repo.canonicalize().unwrap()
        );
        assert_eq!(state.store.repos().unwrap().len(), 1);
        assert!(matches!(
            recording.emitted().as_slice(),
            [
                crate::feedback::Emitted::State(StateChangeKind::WorkspaceStatus),
                crate::feedback::Emitted::Toast(_)
            ]
        ));
        assert_eq!(
            recording.toasts(),
            ["Added app. Add its setup and run commands below."]
        );
    }

    #[tokio::test]
    async fn adding_finds_the_top_of_the_repo_from_a_subfolder() {
        let (state, _recording, out, dir) = fixture();
        let repo = init_repo(&dir, "app");
        let nested = repo.join("src");
        std::fs::create_dir(&nested).unwrap();

        let record = add_path(&out, &state, &nested).await.unwrap();

        assert_eq!(
            expand_tilde(&record.path).canonicalize().unwrap(),
            repo.canonicalize().unwrap()
        );
    }

    #[tokio::test]
    async fn adding_refuses_what_is_not_a_new_repo() {
        let (state, recording, out, dir) = fixture();
        let repo = init_repo(&dir, "app");
        let plain = dir.path().join("plain");
        std::fs::create_dir(&plain).unwrap();
        let other = TempDir::new().unwrap();
        let same_name = init_repo(&other, "app");

        let missing = dir.path().join("missing");
        let error = add_path(&out, &state, &missing).await.unwrap_err();
        assert!(error.to_string().contains("folder does not exist"));
        let error = add_path(&out, &state, &plain).await.unwrap_err();
        assert!(error.to_string().contains("not a git repository"));
        assert!(recording.emitted().is_empty());

        add_path(&out, &state, &repo).await.unwrap();
        let error = add_path(&out, &state, &repo).await.unwrap_err();
        assert!(error.to_string().contains("already added"));
        let error = add_path(&out, &state, &same_name).await.unwrap_err();
        assert!(error.to_string().contains("A repo named app"));
        assert_eq!(state.store.repos().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn removing_forgets_the_repo_and_leaves_the_folder() {
        let (state, recording, out, dir) = fixture();
        let repo = init_repo(&dir, "app");
        register(&state, "app", &repo);
        register(&state, "other", Path::new("/tmp/other"));

        let input = RemoveRepoInput {
            repo_id: "app".into(),
        };
        remove(&out, &state, input).await.unwrap();

        assert!(state.store.repo_by_id("app").unwrap().is_none());
        assert!(repo.is_dir());
        assert_eq!(
            recording.state_changes(),
            [StateChangeKind::WorkspaceStatus]
        );
        assert_eq!(
            recording.toasts(),
            ["Removed app. The folder on disk wasn't touched."]
        );
    }

    #[tokio::test]
    async fn removing_refuses_a_repo_in_use_the_last_repo_and_an_unknown_one() {
        let (state, recording, out, dir) = fixture();
        register(&state, "app", dir.path());
        let input = |repo_id: &str| RemoveRepoInput {
            repo_id: repo_id.into(),
        };

        let error = remove(&out, &state, input("app")).await.unwrap_err();
        assert!(error.to_string().contains("at least one repo"));

        let error = remove(&out, &state, input("nope")).await.unwrap_err();
        assert!(error.to_string().contains("unknown repo nope"));

        register(&state, "other", Path::new("/tmp/other"));
        live_workspace(&state, "app", "feat/x");
        let error = remove(&out, &state, input("app")).await.unwrap_err();
        assert!(error.to_string().contains("app has 1 workspace."));

        assert_eq!(state.store.repos().unwrap().len(), 2);
        assert!(recording.emitted().is_empty());
    }

    #[test]
    fn the_default_branch_cant_be_one_a_workspace_has_checked_out() {
        let (state, recording, out, dir) = fixture();
        register(&state, "app", dir.path());
        live_workspace(&state, "app", "feat/x");
        let input = |branch: &str| SetRepoDefaultBranchInput {
            repo_id: "app".into(),
            default_branch: branch.into(),
        };

        let error = set_default_branch(&out, &state, input("feat/x")).unwrap_err();
        assert!(error.to_string().contains("Work has feat/x checked out"));
        let error = set_default_branch(&out, &state, input("  ")).unwrap_err();
        assert!(error.to_string().contains("Pick a default branch"));
        assert!(recording.emitted().is_empty());
    }

    #[tokio::test]
    async fn a_run_command_is_trimmed_and_a_blank_one_cleared() {
        let (state, _recording, out, dir) = fixture();
        register(&state, "app", dir.path());
        let input = |run_command: &str| SetRepoRunCommandInput {
            repo_id: "app".into(),
            run_command: Some(run_command.into()),
        };

        set_run_command(&out, &state, input("  pnpm dev ")).unwrap();
        let repo = state.store.repo_by_id("app").unwrap().unwrap();
        assert_eq!(repo.run_command.as_deref(), Some("pnpm dev"));

        set_run_command(&out, &state, input("   ")).unwrap();
        let repo = state.store.repo_by_id("app").unwrap().unwrap();
        assert_eq!(repo.run_command, None);
    }

    #[tokio::test]
    async fn deleting_a_branch_guards_what_is_in_use_and_unpushed_work() {
        let (state, _recording, _out, dir) = fixture();
        let repo = init_repo(&dir, "app");
        register(&state, "app", &repo);
        git(&repo, &["branch", "old"]);
        git(&repo, &["branch", "feat/x"]);
        git(&repo, &["switch", "-q", "-c", "current"]);
        live_workspace(&state, "app", "feat/x");

        for (branch, message) in [
            ("  ", "missing branch name"),
            ("main", "is the default branch"),
            ("feat/x", "is checked out by Work"),
            ("current", "is checked out in the repo"),
        ] {
            let error = delete_local_branch(&state, "app", branch, true)
                .await
                .unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
        }

        let kept = delete_local_branch(&state, "app", "old", false).await;
        assert!(matches!(kept, Ok(BranchDeletion::Unpushed)));
        let deleted = delete_local_branch(&state, "app", "old", true).await;
        assert!(matches!(deleted, Ok(BranchDeletion::Deleted)));
        let tips = list_local_branches(&state, "app").await.unwrap();
        assert_eq!(tips.head, "current");
        assert!(!tips.branches.iter().any(|row| row.name == "old"));
    }

    #[tokio::test]
    async fn testing_setup_with_no_commands_passes_without_running_anything() {
        let (state, _recording, _out, dir) = fixture();
        let repo = init_repo(&dir, "app");
        register(&state, "app", &repo);

        let input = TestRepoSetupInput {
            repo_id: "app".into(),
        };
        let result = test_setup(&state, input).await.unwrap();

        assert!(result.ok);
        assert_eq!(result.message, "No setup commands to run.");
    }
}
