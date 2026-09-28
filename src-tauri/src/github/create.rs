use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

use crate::error::{Error, Result};

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
    pub title: String,
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

    pub async fn create_pull_request(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        input: &CreatePullRequestInput,
    ) -> Result<CreatedPullRequest> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
        let body = serde_json::json!({
            "title": input.title,
            "body": input.body,
            "head": input.head,
            "base": input.base,
            "draft": input.draft,
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
                "create pull request failed ({status}): {text}"
            )));
        }

        let payload: CreateResponse = serde_json::from_str(&text)
            .map_err(|error| Error::Github(format!("parse create PR response: {error}")))?;

        Ok(CreatedPullRequest {
            number: payload.number,
            html_url: payload.html_url,
            title: payload.title,
        })
    }
}

#[derive(Debug, Deserialize)]
struct CreateResponse {
    number: i64,
    html_url: String,
    title: String,
}
