use tauri::State;

use crate::error::Result;
use crate::state::AppState;
use crate::store::types::RepoRecord;

use super::{
    AddRepoInput, BranchDeletion, LocalBranchesResult, RemoveRepoInput, RepoBranchesResult,
    RepoGitRuntime, SetRepoDefaultBranchInput, SetRepoRunCommandInput, SetRepoSetupCommandsInput,
    SetRepoSingleInstanceInput, TestRepoSetupInput, TestRepoSetupResult,
};

#[tauri::command]
#[specta::specta]
pub async fn set_repo_run_command(
    input: SetRepoRunCommandInput,
    state: State<'_, AppState>,
) -> Result<()> {
    super::set_run_command(&state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_single_instance(
    input: SetRepoSingleInstanceInput,
    state: State<'_, AppState>,
) -> Result<()> {
    super::set_single_instance(&state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_setup_commands(
    input: SetRepoSetupCommandsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    super::set_setup_commands(&state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_default_branch(
    input: SetRepoDefaultBranchInput,
    state: State<'_, AppState>,
) -> Result<()> {
    super::set_default_branch(&state, input)
}

/// Commits each repo's default branch is behind and ahead of `origin`, against the last
/// fetch. A repo with no `origin` copy of the branch reads as 0 and 0.
#[tauri::command]
#[specta::specta]
pub async fn get_repo_git(state: State<'_, AppState>) -> Result<Vec<RepoGitRuntime>> {
    super::git_runtime(&state).await
}

/// Fast-forward the repo checkout's default branch from origin.
#[tauri::command]
#[specta::specta]
pub async fn pull_repo_default_branch(repo_id: String, state: State<'_, AppState>) -> Result<()> {
    super::pull_default_branch(&state, &repo_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn add_repo(input: AddRepoInput, state: State<'_, AppState>) -> Result<RepoRecord> {
    super::add(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_repo(input: RemoveRepoInput, state: State<'_, AppState>) -> Result<()> {
    super::remove(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn test_repo_setup(
    input: TestRepoSetupInput,
    state: State<'_, AppState>,
) -> Result<TestRepoSetupResult> {
    super::test_setup(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_repo_branches(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<RepoBranchesResult> {
    super::list_branches(&state, &repo_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_repo_local_branches(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<LocalBranchesResult> {
    super::list_local_branches(&state, &repo_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_local_branch(
    repo_id: String,
    branch: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<BranchDeletion> {
    super::delete_local_branch(&state, &repo_id, &branch, force).await
}
