use serde::Deserialize;

use crate::error::{Error, Result};

use super::rest::RestGithubClient;
use super::rest::api_url;

#[derive(Debug, Clone)]
pub struct CreatePullRequestInput {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
}

#[derive(Debug, Clone)]
pub struct CreatedPullRequest {
    pub number: i64,
    pub html_url: String,
}

impl RestGithubClient {
    pub async fn create_pull_request(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        input: &CreatePullRequestInput,
    ) -> Result<CreatedPullRequest> {
        let body = serde_json::json!({
            "title": input.title,
            "body": input.body,
            "head": input.head,
            "base": input.base,
            "draft": input.draft,
        });
        let text = self
            .post(
                token,
                &api_url(&format!("repos/{owner}/{repo}/pulls")),
                &body,
            )
            .await
            .map_err(|error| Error::Github(format!("create pull request failed: {error}")))?;
        let payload: CreateResponse = serde_json::from_str(&text)
            .map_err(|error| Error::Github(format!("parse create PR response: {error}")))?;
        Ok(CreatedPullRequest {
            number: payload.number,
            html_url: payload.html_url,
        })
    }
}

#[derive(Debug, Deserialize)]
struct CreateResponse {
    number: i64,
    html_url: String,
}
