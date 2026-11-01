//! The one REST client behind creating pull requests and submitting reviews: shared headers,
//! a request timeout, and GitHub's error text surfaced as the error.

use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};

use crate::error::{Error, Result};

/// A GitHub call that hasn't answered in this long isn't going to; without it a dropped
/// connection left Create PR spinning for good.
const TIMEOUT: Duration = Duration::from_secs(30);

pub fn api_url(path: &str) -> String {
    format!("https://api.github.com/{path}")
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
            http: reqwest::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .unwrap_or_default(),
        }
    }

    /// POSTs `body` as JSON and returns the response text, or GitHub's own message on failure.
    pub(super) async fn post(
        &self,
        token: &str,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<String> {
        let response = self
            .http
            .post(url)
            .headers(headers(token)?)
            .json(body)
            .send()
            .await
            .map_err(|error| Error::Github(error.to_string()))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| Error::Github(error.to_string()))?;
        if !status.is_success() {
            return Err(Error::Github(format!("({status}): {text}")));
        }
        Ok(text)
    }
}

fn headers(token: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|error| Error::Github(error.to_string()))?,
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("cormux"));
    Ok(headers)
}
