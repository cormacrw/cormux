//! Registered repos: adding and removing them, their settings, and git in their main checkouts.

pub mod commands;
mod store;

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::paths::expand_tilde;
use crate::process::{Exit, HarnessEnv, SessionId, ShellCommand};
use crate::state::AppState;
use crate::store::types::RepoRecord;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoBranchesResult {
    pub branches: Vec<String>,
}

/// A local branch tip. `committed` is git's relative date ("2 hours ago").
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalBranchRow {
    pub name: String,
    pub committed: String,
    pub subject: String,
}

/// What happened to a local branch the user asked to delete.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BranchDeletion {
    Deleted,
    /// Not deleted: origin doesn't have the branch as it is locally, so deleting it
    /// (with `force`) can lose commits.
    Unpushed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalBranchesResult {
    /// Branch checked out in the repo itself. Empty when detached.
    pub head: String,
    pub branches: Vec<LocalBranchRow>,
}

/// How far a repo's local default branch is from `origin`, as of the last fetch.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoGitRuntime {
    pub repo_id: String,
    pub behind: u32,
    pub ahead: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetRepoRunCommandInput {
    pub repo_id: String,
    pub run_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetRepoSingleInstanceInput {
    pub repo_id: String,
    pub single_instance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetRepoSetupCommandsInput {
    pub repo_id: String,
    pub setup_commands: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetRepoDefaultBranchInput {
    pub repo_id: String,
    pub default_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddRepoInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRepoInput {
    pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TestRepoSetupInput {
    pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TestRepoSetupResult {
    pub ok: bool,
    pub message: String,
}

fn repo_paths_match(left: &str, right: &Path) -> bool {
    let left_path = expand_tilde(left);
    left_path.canonicalize().ok() == right.canonicalize().ok()
}

fn upsert_repo_record(state: &AppState, repo: RepoRecord) -> Result<()> {
    state.store.upsert_repo(&repo)?;
    state.out.changed();
    Ok(())
}

pub fn set_run_command(state: &AppState, input: SetRepoRunCommandInput) -> Result<()> {
    let mut repo = state.store.repo(&input.repo_id)?;
    repo.run_command = input
        .run_command
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    upsert_repo_record(state, repo)
}

pub fn set_single_instance(state: &AppState, input: SetRepoSingleInstanceInput) -> Result<()> {
    let mut repo = state.store.repo(&input.repo_id)?;
    repo.single_instance = input.single_instance;
    upsert_repo_record(state, repo)
}

pub fn set_setup_commands(state: &AppState, input: SetRepoSetupCommandsInput) -> Result<()> {
    let mut repo = state.store.repo(&input.repo_id)?;
    repo.setup_commands = input.setup_commands;
    upsert_repo_record(state, repo)
}

pub fn set_default_branch(state: &AppState, input: SetRepoDefaultBranchInput) -> Result<()> {
    let mut repo = state.store.repo(&input.repo_id)?;
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
    repo.default_branch = branch;
    state.store.upsert_repo(&repo)?;
    state.out.repo_git_moved();
    Ok(())
}

/// Commits each repo's default branch is behind and ahead of `origin`, against the last
/// fetch. A repo with no `origin` copy of the branch reads as 0 and 0.
pub async fn git_runtime(state: &AppState) -> Result<Vec<RepoGitRuntime>> {
    let mut rows = Vec::new();
    for repo in state.store.repos()? {
        let path = expand_tilde(&repo.path);
        let branch = repo.default_branch.as_str();
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

/// A path typed into "Add repo", without its trailing slashes, and the id its last folder
/// name gives the repo.
fn validate_add_path(path: &str) -> Result<(String, String)> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(Error::Workspace("Enter the path to a folder".into()));
    }
    let normalized = trimmed.trim_end_matches('/').to_string();
    let segment = normalized.rsplit('/').next().unwrap_or("").trim();
    let absolute = normalized.starts_with("~/") || normalized.starts_with('/');
    if !absolute || segment.is_empty() || segment == "~" {
        return Err(Error::Workspace(
            "Use a full path, like ~/code/my-project".into(),
        ));
    }
    let id = segment.to_string();
    Ok((normalized, id))
}

pub async fn add(state: &AppState, input: AddRepoInput) -> Result<RepoRecord> {
    let (normalized, id) = validate_add_path(&input.path)?;
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
    let display_path = crate::paths::for_display(&toplevel);

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

    let default_branch = state
        .git
        .default_branch(&toplevel)
        .await
        .unwrap_or_else(|_| crate::model::FALLBACK_DEFAULT_BRANCH.into());
    let committed = match crate::harness_config::read_committed_config(&toplevel) {
        Ok(committed) => committed,
        Err(error) => {
            log::warn!("adding {id}: {error}");
            state.out.toast(ToastRaisedPayload {
                tone: ToastTone::Bad,
                parts: vec![ToastPart::Text {
                    value: format!("{error}. Using suggested commands instead."),
                }],
                workspace_id: None,
                thread_id: None,
            });
            None
        }
    };
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
    state.store.upsert_repo(&record)?;
    state.out.repo_git_moved();

    state.out.toast(ToastRaisedPayload {
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
    resync_prs(state);

    Ok(record)
}

pub async fn remove(state: &AppState, input: RemoveRepoInput) -> Result<()> {
    let repo = state.store.repo(&input.repo_id)?;

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
    state.out.changed();

    state.out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![ToastPart::Text {
            value: format!("Removed {}. The folder on disk wasn't touched.", repo.name),
        }],
        workspace_id: None,
        thread_id: None,
    });
    resync_prs(state);
    Ok(())
}

/// The PR list only shows registered repos, so re-sync when that set changes.
fn resync_prs(state: &AppState) {
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = state.pr_sync.sync_app(&state).await {
            log::warn!("PR sync after repo change failed: {error}");
        }
    });
}

pub async fn test_setup(
    state: &AppState,
    input: TestRepoSetupInput,
) -> Result<TestRepoSetupResult> {
    let repo = state.store.repo(&input.repo_id)?;
    let repo_path = expand_tilde(&repo.path);
    let setup_raw = crate::harness_config::effective_setup(&repo.setup_commands, &repo_path)?;
    let commands = crate::provisioning::parse_setup_commands(&setup_raw);
    if commands.is_empty() {
        return Ok(TestRepoSetupResult {
            ok: true,
            message: "No setup commands to run.".into(),
        });
    }

    let env = checkout_env(state, &repo, &repo_path).await.vars();
    for (index, command) in commands.iter().enumerate() {
        let session = SessionId::RepoTest {
            repo: repo.id.clone(),
            index,
        };
        let shell = ShellCommand {
            cmd: command.clone(),
            cwd: repo_path.clone(),
            env: env.clone(),
        };
        let exit = state
            .process
            .run_to_exit(
                session,
                &shell,
                crate::provisioning::SETUP_COMMAND_TIMEOUT,
                None,
            )
            .await?;
        let message = match exit {
            Exit::Code(0) => continue,
            Exit::Code(code) => format!("Command failed ({code}): {command}"),
            Exit::TimedOut => format!("Command timed out: {command}"),
        };
        return Ok(TestRepoSetupResult { ok: false, message });
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

/// The env setup gets when it runs in the repo's own checkout, which is then both the repo and
/// the worktree, on whatever branch is checked out there.
async fn checkout_env(state: &AppState, repo: &RepoRecord, repo_path: &Path) -> HarnessEnv {
    let base = repo.default_branch.clone();
    let branch = state
        .git
        .current_branch(repo_path)
        .await
        .ok()
        .filter(|branch| !branch.is_empty())
        .unwrap_or_else(|| base.clone());
    let path = repo_path.to_string_lossy().to_string();
    HarnessEnv {
        repo_path: path.clone(),
        worktree_path: path,
        workspace: repo.name.clone(),
        branch,
        base,
    }
}

pub async fn list_branches(state: &AppState, repo_id: &str) -> Result<RepoBranchesResult> {
    let repo = state.store.repo(repo_id)?;
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
    let repo = state.store.repo(repo_id)?;
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
    let repo = state.store.repo(repo_id)?;
    if branch == repo.default_branch {
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

    fn fixture() -> (AppState, Arc<RecordingOutput>, TempDir) {
        let dir = TempDir::new().unwrap();
        let (state, recording) =
            AppState::recorded(crate::paths::DataDir::new(dir.path().join("data")));
        (state, recording, dir)
    }

    fn register(state: &AppState, id: &str, path: &Path) {
        state
            .store
            .upsert_repo(&RepoRecord {
                id: id.into(),
                path: path.to_string_lossy().to_string(),
                name: id.into(),
                default_branch: "main".into(),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
    }

    fn live_workspace(state: &AppState, repo_id: &str, branch: &str) {
        state
            .store
            .insert_workspace(&WorkspaceRow {
                status: crate::model::WorkspaceLifecycle::Ready,
                ..crate::store::types::WorkspaceRow::new(
                    crate::model::WorkspaceKind::Lead,
                    "w1",
                    repo_id,
                    "Work",
                    branch,
                    "main",
                    "/tmp/wt",
                )
            })
            .unwrap();
    }

    async fn add_path(state: &AppState, path: &Path) -> Result<RepoRecord> {
        let input = AddRepoInput {
            path: path.to_string_lossy().to_string(),
        };
        add(state, input).await
    }

    #[tokio::test]
    async fn adding_a_repo_records_it_then_refreshes_and_toasts() {
        let (state, recording, dir) = fixture();
        let repo = init_repo(&dir, "app");

        let record = add_path(&state, &repo).await.unwrap();

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
                crate::feedback::Emitted::State(_),
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
        let (state, _recording, dir) = fixture();
        let repo = init_repo(&dir, "app");
        let nested = repo.join("src");
        std::fs::create_dir(&nested).unwrap();

        let record = add_path(&state, &nested).await.unwrap();

        assert_eq!(
            expand_tilde(&record.path).canonicalize().unwrap(),
            repo.canonicalize().unwrap()
        );
    }

    #[tokio::test]
    async fn adding_refuses_what_is_not_a_new_repo() {
        let (state, recording, dir) = fixture();
        let repo = init_repo(&dir, "app");
        let plain = dir.path().join("plain");
        std::fs::create_dir(&plain).unwrap();
        let other = TempDir::new().unwrap();
        let same_name = init_repo(&other, "app");

        let missing = dir.path().join("missing");
        let error = add_path(&state, &missing).await.unwrap_err();
        assert!(error.to_string().contains("folder does not exist"));
        let error = add_path(&state, &plain).await.unwrap_err();
        assert!(error.to_string().contains("not a git repository"));
        assert!(recording.emitted().is_empty());

        add_path(&state, &repo).await.unwrap();
        let error = add_path(&state, &repo).await.unwrap_err();
        assert!(error.to_string().contains("already added"));
        let error = add_path(&state, &same_name).await.unwrap_err();
        assert!(error.to_string().contains("A repo named app"));
        assert_eq!(state.store.repos().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn removing_forgets_the_repo_and_leaves_the_folder() {
        let (state, recording, dir) = fixture();
        let repo = init_repo(&dir, "app");
        register(&state, "app", &repo);
        register(&state, "other", Path::new("/tmp/other"));

        let input = RemoveRepoInput {
            repo_id: "app".into(),
        };
        remove(&state, input).await.unwrap();

        assert!(state.store.repo_by_id("app").unwrap().is_none());
        assert!(repo.is_dir());
        assert_eq!(recording.state_changes().len(), 1);
        assert_eq!(
            recording.toasts(),
            ["Removed app. The folder on disk wasn't touched."]
        );
    }

    #[tokio::test]
    async fn removing_refuses_a_repo_in_use_the_last_repo_and_an_unknown_one() {
        let (state, recording, dir) = fixture();
        register(&state, "app", dir.path());
        let input = |repo_id: &str| RemoveRepoInput {
            repo_id: repo_id.into(),
        };

        let error = remove(&state, input("app")).await.unwrap_err();
        assert!(error.to_string().contains("at least one repo"));

        let error = remove(&state, input("nope")).await.unwrap_err();
        assert!(error.to_string().contains("unknown repo nope"));

        register(&state, "other", Path::new("/tmp/other"));
        live_workspace(&state, "app", "feat/x");
        let error = remove(&state, input("app")).await.unwrap_err();
        assert!(error.to_string().contains("app has 1 workspace."));

        assert_eq!(state.store.repos().unwrap().len(), 2);
        assert!(recording.emitted().is_empty());
    }

    #[test]
    fn the_default_branch_cant_be_one_a_workspace_has_checked_out() {
        let (state, recording, dir) = fixture();
        register(&state, "app", dir.path());
        live_workspace(&state, "app", "feat/x");
        let input = |branch: &str| SetRepoDefaultBranchInput {
            repo_id: "app".into(),
            default_branch: branch.into(),
        };

        let error = set_default_branch(&state, input("feat/x")).unwrap_err();
        assert!(error.to_string().contains("Work has feat/x checked out"));
        let error = set_default_branch(&state, input("  ")).unwrap_err();
        assert!(error.to_string().contains("Pick a default branch"));
        assert!(recording.emitted().is_empty());
    }

    #[tokio::test]
    async fn a_run_command_is_trimmed_and_a_blank_one_cleared() {
        let (state, _recording, dir) = fixture();
        register(&state, "app", dir.path());
        let input = |run_command: &str| SetRepoRunCommandInput {
            repo_id: "app".into(),
            run_command: Some(run_command.into()),
        };

        set_run_command(&state, input("  pnpm dev ")).unwrap();
        let repo = state.store.repo_by_id("app").unwrap().unwrap();
        assert_eq!(repo.run_command.as_deref(), Some("pnpm dev"));

        set_run_command(&state, input("   ")).unwrap();
        let repo = state.store.repo_by_id("app").unwrap().unwrap();
        assert_eq!(repo.run_command, None);
    }

    #[tokio::test]
    async fn deleting_a_branch_guards_what_is_in_use_and_unpushed_work() {
        let (state, _recording, dir) = fixture();
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
        let (state, _recording, dir) = fixture();
        let repo = init_repo(&dir, "app");
        register(&state, "app", &repo);

        let input = TestRepoSetupInput {
            repo_id: "app".into(),
        };
        let result = test_setup(&state, input).await.unwrap();

        assert!(result.ok);
        assert_eq!(result.message, "No setup commands to run.");
    }

    #[tokio::test]
    async fn testing_setup_runs_in_the_checkout_with_its_branch_and_the_full_env() {
        let (state, _recording, dir) = fixture();
        let repo = init_repo(&dir, "app");
        git(&repo, &["switch", "-q", "-c", "feat/x"]);
        register(&state, "app", &repo);
        let mut record = state.store.repo_by_id("app").unwrap().unwrap();
        record.setup_commands = [
            r#"test "$HARNESS_BRANCH" = feat/x"#,
            r#"test "$HARNESS_BASE" = main && test "$HARNESS_WORKSPACE" = app"#,
            r#"test -n "$HARNESS_WORKTREE_PATH" && test "$HARNESS_REPO_PATH" = "$HARNESS_WORKTREE_PATH""#,
            "exit 4",
        ]
        .join("\n");
        state.store.upsert_repo(&record).unwrap();

        let input = TestRepoSetupInput {
            repo_id: "app".into(),
        };
        let result = test_setup(&state, input).await.unwrap();

        assert!(!result.ok);
        assert_eq!(result.message, "Command failed (4): exit 4");
    }

    #[tokio::test]
    async fn a_broken_committed_config_is_reported_not_skipped() {
        let (state, recording, dir) = fixture();
        let repo = init_repo(&dir, "app");
        std::fs::create_dir(repo.join(".cormux")).unwrap();
        std::fs::write(repo.join(".cormux/config.json"), "{ nope").unwrap();

        let record = add_path(&state, &repo).await.unwrap();
        assert_eq!(record.id, "app");
        assert!(
            recording.toasts()[0].contains(".cormux/config.json can't be used"),
            "{:?}",
            recording.toasts()
        );

        let input = TestRepoSetupInput {
            repo_id: "app".into(),
        };
        let error = test_setup(&state, input).await.unwrap_err();
        assert!(error.to_string().contains("can't be used"), "{error}");
    }

    #[test]
    fn an_added_path_must_be_absolute_or_under_home() {
        assert!(validate_add_path("  ").is_err());
        assert!(validate_add_path("relative/path").is_err());
        assert!(validate_add_path("~code/app").is_err());
        assert!(validate_add_path("~/").is_err());
        let (path, id) = validate_add_path("~/code/my-app/").unwrap();
        assert_eq!((path.as_str(), id.as_str()), ("~/code/my-app", "my-app"));
        let (path, id) = validate_add_path("/srv/app").unwrap();
        assert_eq!((path.as_str(), id.as_str()), ("/srv/app", "app"));
    }
}
