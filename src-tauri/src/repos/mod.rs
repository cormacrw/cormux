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
    pub branches: Vec<RepoBranch>,
}

/// A branch a workspace can check out, local or only on `origin`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoBranch {
    pub name: String,
    /// Why no workspace can take it now.
    pub hold: Option<BranchHold>,
}

/// A local branch tip. `committed` is git's relative date ("2 hours ago").
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalBranchRow {
    pub name: String,
    pub committed: String,
    pub subject: String,
    pub hold: Option<BranchHold>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BranchHold {
    DefaultBranch,
    /// Live workspaces have it checked out, by name.
    CheckedOutBy {
        workspaces: Vec<String>,
    },
    /// The repo's own checkout is on it.
    RepoHead,
}

/// What holds a repo's branches where they are: its default branch, the branches live
/// workspaces have checked out, and the one its own checkout is on. A held branch can't be
/// deleted, and no workspace can switch to it.
pub struct BranchHolds {
    repo: RepoRecord,
    live: Vec<crate::store::types::WorkspaceRow>,
    /// The branch the repo's checkout is on; '' when detached.
    head: String,
}

impl BranchHolds {
    pub async fn read(state: &AppState, repo: RepoRecord) -> Result<Self> {
        let head = state
            .git
            .current_branch(&expand_tilde(&repo.path))
            .await
            .unwrap_or_default();
        Ok(Self {
            head,
            ..Self::recorded(state, repo)?
        })
    }

    /// Without the repo checkout's branch, which takes a git call: for checks that run before
    /// any git call, or on names the checkout's branch can't hold.
    pub fn recorded(state: &AppState, repo: RepoRecord) -> Result<Self> {
        Ok(Self {
            live: state.store.live_workspaces()?,
            ..Self::of_repo(repo)
        })
    }

    /// The repo's own hold alone, its default branch: for checks made without the store.
    pub fn of_repo(repo: RepoRecord) -> Self {
        Self {
            repo,
            live: Vec::new(),
            head: String::new(),
        }
    }

    /// As `workspace_id` sees them: its own branch doesn't hold it.
    pub fn seen_from(mut self, workspace_id: &str) -> Self {
        self.live.retain(|row| row.id != workspace_id);
        self
    }

    /// A name typed with spaces around it names the same branch.
    pub fn of(&self, branch: &str) -> Option<BranchHold> {
        let branch = branch.trim();
        if branch == self.repo.default_branch {
            return Some(BranchHold::DefaultBranch);
        }
        let workspaces: Vec<String> = self
            .live
            .iter()
            .filter(|row| row.repo_id == self.repo.id && row.branch == branch)
            .map(|row| row.name.clone())
            .collect();
        if !workspaces.is_empty() {
            return Some(BranchHold::CheckedOutBy { workspaces });
        }
        (!self.head.is_empty() && self.head == branch).then_some(BranchHold::RepoHead)
    }

    /// The default branch stays in the repo checkout. Git lets a branch be checked out in one
    /// worktree only, so no workspace takes it.
    pub fn ensure_not_default(&self, branch: &str) -> Result<()> {
        if self.of(branch) == Some(BranchHold::DefaultBranch) {
            return Err(Error::Invalid(format!(
                "Workspaces can't check out {}, the default branch of {}. Use another branch.",
                self.repo.default_branch, self.repo.name
            )));
        }
        Ok(())
    }
}

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
    if let Some(BranchHold::CheckedOutBy { workspaces }) =
        BranchHolds::recorded(state, repo.clone())?.of(&branch)
    {
        return Err(Error::Workspace(format!(
            "{} has {branch} checked out. Switch it to another branch first.",
            workspaces.join(", ")
        )));
    }
    repo.default_branch = branch;
    state.store.upsert_repo(&repo)?;
    state.out.repo_git_moved();
    Ok(())
}

/// Commits each repo's default branch is behind and ahead of `origin`, against the last
/// fetch. A repo with no `origin` copy of the branch, or that git can't read (its folder
/// moved), reads as 0 and 0, so one repo can't stop the others' counts.
pub async fn git_runtime(state: &AppState) -> Result<Vec<RepoGitRuntime>> {
    let mut rows = Vec::new();
    for repo in state.store.repos()? {
        let path = expand_tilde(&repo.path);
        let branch = repo.default_branch.as_str();
        let (ahead, behind) = state
            .git
            .ahead_behind(&path, branch, &format!("origin/{branch}"))
            .await
            .unwrap_or_else(|error| {
                log::warn!("counting {}'s commits against origin: {error}", repo.id);
                (0, 0)
            });
        rows.push(RepoGitRuntime {
            repo_id: repo.id,
            behind,
            ahead,
        });
    }
    Ok(rows)
}

/// A path typed into "Add repo", without its trailing slashes.
fn validate_add_path(path: &str) -> Result<String> {
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
    Ok(normalized)
}

pub async fn add(state: &AppState, input: AddRepoInput) -> Result<RepoRecord> {
    let normalized = validate_add_path(&input.path)?;
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
    let id = crate::paths::repo_dir_name(&toplevel).to_string();

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
                ..Default::default()
            });
            None
        }
    };
    let suggested = crate::harness_config::suggest_commands(&toplevel);
    let (setup_commands, run_command) =
        crate::harness_config::initial_commands(committed.as_ref(), suggested);

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
        ..Default::default()
    });
    resync_prs(state);

    Ok(record)
}

pub fn removal_block(repo: &RepoRecord, live_workspaces: usize, repos: usize) -> Option<String> {
    if live_workspaces > 0 {
        let tear = if live_workspaces == 1 { "it" } else { "them" };
        return Some(format!(
            "{} has {live_workspaces} workspace{}. Tear {tear} down first.",
            repo.name,
            if live_workspaces == 1 { "" } else { "s" }
        ));
    }
    (repos <= 1).then(|| "Cormux needs at least one repo".to_string())
}

/// A repo the webview should offer no Remove for, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoRemovalBlock {
    pub repo_id: String,
    pub reason: String,
}

pub fn removal_blocks(
    repos: &[RepoRecord],
    live: &[crate::store::types::WorkspaceRow],
) -> Vec<RepoRemovalBlock> {
    repos
        .iter()
        .filter_map(|repo| {
            let used = live.iter().filter(|row| row.repo_id == repo.id).count();
            removal_block(repo, used, repos.len()).map(|reason| RepoRemovalBlock {
                repo_id: repo.id.clone(),
                reason,
            })
        })
        .collect()
}

pub async fn remove(state: &AppState, input: RemoveRepoInput) -> Result<()> {
    let repo = state.store.repo(&input.repo_id)?;
    let used = state.store.count_active_workspaces_for_repo(&repo.id)?;
    if let Some(reason) = removal_block(&repo, used, state.store.repo_count()?) {
        return Err(Error::Workspace(reason));
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
        ..Default::default()
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
    let holds = BranchHolds::read(state, repo).await?;
    let branches = names
        .into_iter()
        .map(|name| RepoBranch {
            hold: holds.of(&name),
            name,
        })
        .collect();
    Ok(RepoBranchesResult { branches })
}

pub async fn list_local_branches(state: &AppState, repo_id: &str) -> Result<LocalBranchesResult> {
    let repo = state.store.repo(repo_id)?;
    let repo_path = expand_tilde(&repo.path);
    let (holds, tips) = tokio::join!(
        BranchHolds::read(state, repo),
        state.git.list_local_branch_tips(&repo_path),
    );
    let holds = holds?;
    let branches = tips?
        .into_iter()
        .map(|tip| LocalBranchRow {
            hold: holds.of(&tip.name),
            name: tip.name,
            committed: tip.committed,
            subject: tip.subject,
        })
        .collect();
    Ok(LocalBranchesResult {
        head: holds.head,
        branches,
    })
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
    let repo_path = expand_tilde(&repo.path);
    if let Some(hold) = BranchHolds::read(state, repo).await?.of(branch) {
        return Err(Error::Git(match hold {
            BranchHold::DefaultBranch => format!("{branch} is the default branch"),
            BranchHold::CheckedOutBy { workspaces } => {
                format!("{branch} is checked out by {}", workspaces.join(", "))
            }
            BranchHold::RepoHead => format!("{branch} is checked out in the repo"),
        }));
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

    /// Bug: one repo whose folder had moved failed the whole list, so no repo's counts updated.
    #[tokio::test]
    async fn a_repo_git_cant_read_counts_as_zero_without_stopping_the_others() {
        let (state, _recording, dir) = fixture();
        let origin = init_repo(&dir, "origin");
        git(
            dir.path(),
            &["clone", "-q", &origin.to_string_lossy(), "app"],
        );
        let app = dir.path().join("app");
        git(&app, &["config", "user.email", "cormux@test"]);
        git(&app, &["config", "user.name", "Cormux"]);
        git(&app, &["commit", "-q", "--allow-empty", "-m", "local"]);
        register(&state, "app", &app);
        register(&state, "moved", &dir.path().join("moved"));

        let mut rows = git_runtime(&state).await.unwrap();
        rows.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));
        let counts: Vec<_> = rows
            .iter()
            .map(|row| (row.repo_id.as_str(), row.ahead, row.behind))
            .collect();
        assert_eq!(counts, [("app", 1, 0), ("moved", 0, 0)]);
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
        assert!(!recording.state_changes().is_empty());
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
        assert_eq!((record.id.as_str(), record.name.as_str()), ("app", "app"));
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
        assert!(!recording.state_changes().is_empty());
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

    /// Bug: the default-branch refusal trimmed the name and the branch holds didn't, so a name
    /// typed with a space was the default branch to one and free to the other. Every check
    /// goes by the holds now, which trim.
    #[test]
    fn a_branch_is_held_by_one_rule_however_its_name_is_typed() {
        let (state, _recording, dir) = fixture();
        register(&state, "app", dir.path());
        live_workspace(&state, "app", "feat/x");
        let repo = state.store.repo("app").unwrap();
        let holds = BranchHolds::recorded(&state, repo.clone()).unwrap();
        let work = Some(BranchHold::CheckedOutBy {
            workspaces: vec!["Work".into()],
        });

        assert_eq!(holds.of(" main "), Some(BranchHold::DefaultBranch));
        assert_eq!(holds.of("feat/x "), work);
        let error = BranchHolds::of_repo(repo)
            .ensure_not_default("main ")
            .unwrap_err();
        assert!(error.to_string().contains("the default branch"), "{error}");
        assert_eq!(holds.seen_from("w1").of("feat/x"), None);
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

    #[test]
    fn a_repo_with_live_workspaces_or_the_only_repo_cannot_be_removed() {
        let repos = [
            crate::store::fixtures::repo("r1"),
            crate::store::fixtures::repo("r2"),
        ];
        let live = [crate::store::fixtures::workspace(
            "w1",
            crate::model::WorkspaceLifecycle::Idle,
        )];
        assert_eq!(
            removal_blocks(&repos, &live),
            [RepoRemovalBlock {
                repo_id: "r1".into(),
                reason: "r1 has 1 workspace. Tear it down first.".into(),
            }]
        );
        assert_eq!(
            removal_block(&repos[1], 0, 1).as_deref(),
            Some("Cormux needs at least one repo")
        );
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
        // The list says what the delete would refuse, by the same rule.
        let hold = |name: &str| {
            tips.branches
                .iter()
                .find(|row| row.name == name)
                .unwrap()
                .hold
                .clone()
        };
        assert_eq!(hold("main"), Some(BranchHold::DefaultBranch));
        assert_eq!(
            hold("feat/x"),
            Some(BranchHold::CheckedOutBy {
                workspaces: vec!["Work".into()]
            })
        );
        assert_eq!(hold("current"), Some(BranchHold::RepoHead));

        // Bug: the branch picker worked holds out itself and missed the repo's own checkout,
        // offering a branch `git switch` then refused. It gets the same holds now.
        let picker = list_branches(&state, "app").await.unwrap();
        let picked = |name: &str| {
            picker
                .branches
                .iter()
                .find(|row| row.name == name)
                .unwrap()
                .hold
                .clone()
        };
        for name in ["main", "feat/x", "current"] {
            assert_eq!(picked(name), hold(name), "{name}");
        }
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

    /// Bug: adding a repo copied its committed config into Settings, so a later change to the
    /// file never took effect and the run command read as the repo's own, not the config's.
    #[tokio::test]
    async fn an_added_repo_follows_its_committed_config_live() {
        let (state, _recording, dir) = fixture();
        let repo = init_repo(&dir, "app");
        std::fs::create_dir(repo.join(".cormux")).unwrap();
        let commit = |setup: &str| {
            std::fs::write(
                repo.join(".cormux/config.json"),
                format!(r#"{{"setupCommands":"{setup}","runCommand":"pnpm dev"}}"#),
            )
            .unwrap();
        };
        commit("pnpm install");

        let record = add_path(&state, &repo).await.unwrap();
        assert_eq!(record.setup_commands, "");
        assert_eq!(record.run_command, None);
        let run = crate::harness_config::repo_run_command(&record)
            .unwrap()
            .unwrap();
        assert_eq!(run.source, crate::harness_config::RunCommandSource::Config);

        commit("pnpm install --frozen-lockfile");
        let setup = crate::harness_config::effective_setup(&record.setup_commands, &repo).unwrap();
        assert_eq!(setup, "pnpm install --frozen-lockfile");
    }

    #[test]
    fn an_added_path_must_be_absolute_or_under_home() {
        assert!(validate_add_path("  ").is_err());
        assert!(validate_add_path("relative/path").is_err());
        assert!(validate_add_path("~code/app").is_err());
        assert!(validate_add_path("~/").is_err());
        assert_eq!(
            validate_add_path("~/code/my-app/").unwrap(),
            "~/code/my-app"
        );
        assert_eq!(validate_add_path("/srv/app").unwrap(), "/srv/app");
    }
}
