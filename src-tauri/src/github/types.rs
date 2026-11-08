use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PrChecksState {
    Pass,
    Fail,
    Running,
    /// The PR has no status checks or check runs at all.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PrReviewState {
    Required,
    Changes,
    Approved,
    Draft,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestPayload {
    pub num: i64,
    pub title: String,
    /// GitHub login, or [`VIEWER_AUTHOR`] when the viewer authored the PR.
    pub author: String,
    pub head: String,
    pub base: String,
    pub updated_at: String,
    pub checks: PrChecksState,
    pub review: PrReviewState,
    pub files: i64,
    pub is_draft: bool,
    pub html_url: String,
    pub repo_full_name: String,
    pub repo_id: Option<String>,
}

/// The author of a pull request the signed-in user opened, as the PR list shows it.
pub const VIEWER_AUTHOR: &str = "you";

impl PullRequestPayload {
    pub fn cache_id(&self) -> String {
        format!("{}#{}", self.repo_full_name, self.num)
    }

    pub fn authored_by_viewer(&self) -> bool {
        self.author == VIEWER_AUTHOR
    }
}
