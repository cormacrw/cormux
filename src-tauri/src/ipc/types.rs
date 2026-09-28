use serde::{Deserialize, Serialize};
use specta::Type;

use crate::git::{BehindUpdate, WorktreeDiff};
use crate::metrics::MemorySample;
use crate::store::types::PersistedSnapshot;
use crate::app::WorkspaceAppRuntime;
use crate::workspace::{WorkspaceLifecycle, WorkspaceRecord};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u64,
    pub view: AppView,
    pub persisted: PersistedSnapshot,
    pub workspaces: Vec<WorkspaceRecord>,
    pub workspace_git: Vec<WorkspaceGitRuntime>,
    pub memory: Option<MemorySample>,
    /// In-memory broker queue (may exceed persisted pending rows).
    pub pending_live_approvals: usize,
    /// True when a PAT is stored or `gh auth token` succeeds.
    pub github_auth_configured: bool,
    /// Unix seconds string from the last successful PR sync, if any.
    pub pr_synced_at: Option<String>,
    pub workspace_apps: Vec<WorkspaceAppRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AppView {
    Homebase,
    Workspace { id: String },
    Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolveApprovalResult {
    pub focus_composer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentChunk {
    pub thread_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PtyChunk {
    pub workspace_id: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffUpdate {
    pub workspace_id: String,
    pub path: String,
    pub diff: Option<WorktreeDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StateChangeKind {
    WorkspaceStatus,
    ApprovalCounts,
    PrSync,
    Toast,
    BehindCounts,
    Metrics,
    Environment,
    WorkspaceApp,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ToastTone {
    Ok,
    Bad,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ToastPart {
    Text { value: String },
    Code { value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ToastRaisedPayload {
    pub tone: ToastTone,
    pub parts: Vec<ToastPart>,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatusPayload {
    pub version: u64,
    pub workspace: WorkspaceRecord,
    pub status: WorkspaceLifecycle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BehindCountsPayload {
    pub version: u64,
    pub updates: Vec<BehindUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummaryResult {
    pub workspace_id: String,
    pub summary: String,
    pub summary_at: String,
    pub summary_source: String,
    pub from_llm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub repo_id: String,
    pub name: String,
    pub branch: String,
    pub base: String,
    pub engine: String,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceResult {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoBranchesResult {
    pub branches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitRuntime {
    pub workspace_id: String,
    pub behind: u32,
    pub ahead: u32,
    pub conflict: Option<GitConflictState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GitConflictState {
    pub operation: GitConflictOperation,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GitConflictOperation {
    Merge,
    Rebase,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SwitchWorkspaceBranchInput {
    pub workspace_id: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceBranchInput {
    pub workspace_id: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JoinWorkspaceThreadInput {
    pub workspace_id: String,
    pub title: String,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SendWorkspaceFindingsInput {
    pub workspace_id: String,
    pub thread_id: String,
    pub finding_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JoinWorkspaceThreadResult {
    pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorkspaceInput {
    pub workspace_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ControlWorkspaceAppInput {
    pub workspace_id: String,
    pub action: WorkspaceAppControlAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceAppControlAction {
    Run,
    Restart,
    Stop,
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetRepoRunCommandInput {
    pub repo_id: String,
    pub run_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetRepoSetupCommandsInput {
    pub repo_id: String,
    pub setup_commands: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetSettingInput {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DraftPrWhyResult {
    pub workspace_id: String,
    pub text: String,
    pub from_llm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspacePullRequestInput {
    pub workspace_id: String,
    pub why: String,
    pub title: Option<String>,
    pub draft: bool,
    pub include_what_changed: bool,
    pub include_how_tested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspacePullRequestResult {
    pub workspace_id: String,
    pub number: i64,
    pub html_url: String,
    pub title: String,
}

pub use crate::engines::{AgentEvent, MessageRole, PlanStep, ToolCallStatus, ToolKind};
pub use crate::teardown::{TeardownInput, TeardownPreview};
