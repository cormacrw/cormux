use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PrRelationship {
    Review,
    Author,
    Mention,
    Assigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PrChecksState {
    Pass,
    Fail,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PrReviewState {
    Required,
    Changes,
    Approved,
    Draft,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestPayload {
    pub num: i64,
    pub title: String,
    /// GitHub login, or the literal `you` when the viewer authored the PR.
    pub author: String,
    pub rel: PrRelationship,
    pub head: String,
    pub base: String,
    pub updated_at: String,
    pub checks: PrChecksState,
    pub failing: Option<u32>,
    pub review: PrReviewState,
    pub additions: i64,
    pub deletions: i64,
    pub files: i64,
    pub is_draft: bool,
    pub html_url: String,
    pub repo_full_name: String,
    pub repo_id: Option<String>,
}

impl PullRequestPayload {
    pub fn cache_id(&self) -> String {
        format!("{}#{}", self.repo_full_name, self.num)
    }
}
