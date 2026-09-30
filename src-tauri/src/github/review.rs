use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

use crate::error::{Error, Result};

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

#[derive(Clone)]
pub struct RestGithubClient {
    http: reqwest::Client,
}

impl Default for RestGithubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RestGithubClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn submit_pull_request_review(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        pull_number: i64,
        input: &SubmitPullRequestReviewInput,
    ) -> Result<()> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls/{pull_number}/reviews");
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

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|error| Error::Github(error.to_string()))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("cormux"));

        let response = self
            .http
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|error| Error::Github(error.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| Error::Github(error.to_string()))?;

        if !status.is_success() {
            return Err(Error::Github(format!(
                "submit pull request review failed ({status}): {text}"
            )));
        }

        let _payload: SubmitResponse = serde_json::from_str(&text)
            .map_err(|error| Error::Github(format!("parse review response: {error}")))?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct SubmitResponse {
    #[allow(dead_code)]
    id: i64,
}
