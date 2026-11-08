use crate::error::{Error, Result};

use super::rest::RestGithubClient;
use super::rest::api_url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}

impl ReviewVerdict {
    pub fn as_api_event(self) -> &'static str {
        match self {
            Self::Approve => "APPROVE",
            Self::RequestChanges => "REQUEST_CHANGES",
            Self::Comment => "COMMENT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReviewLineComment {
    pub path: String,
    pub line: i64,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct SubmitPullRequestReviewInput {
    pub verdict: ReviewVerdict,
    pub body: String,
    pub comments: Vec<ReviewLineComment>,
}

impl RestGithubClient {
    pub async fn submit_pull_request_review(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        pull_number: i64,
        input: &SubmitPullRequestReviewInput,
    ) -> Result<()> {
        let comments: Vec<serde_json::Value> = input
            .comments
            .iter()
            .map(|comment| {
                serde_json::json!({
                    "path": comment.path,
                    "line": comment.line,
                    "side": "RIGHT",
                    "body": comment.body,
                })
            })
            .collect();
        let body = serde_json::json!({
            "event": input.verdict.as_api_event(),
            "body": input.body,
            "comments": comments,
        });
        let url = api_url(&format!("repos/{owner}/{repo}/pulls/{pull_number}/reviews"));
        self.post(token, &url, &body).await.map_err(|error| {
            Error::Github(format!("submit pull request review failed: {error}"))
        })?;
        Ok(())
    }
}
