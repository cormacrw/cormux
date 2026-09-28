use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

use crate::error::{Error, Result};

use super::types::{
    PrChecksState, PrRelationship, PrReviewState, PullRequestPayload,
};

const GRAPHQL_URL: &str = "https://api.github.com/graphql";

const SEARCH_QUERY: &str = r#"
query($q: String!) {
  viewer { login }
  search(query: $q, type: ISSUE, first: 100) {
    nodes {
      ... on PullRequest {
        number
        title
        isDraft
        url
        updatedAt
        additions
        deletions
        changedFiles
        reviewDecision
        author { login }
        headRefName
        baseRefName
        repository { nameWithOwner }
        statusCheckRollup { state }
        requestedReviewers(first: 20) {
          nodes { ... on User { login } }
        }
        assignees(first: 20) {
          nodes { login }
        }
      }
    }
  }
}
"#;

#[derive(Debug, Clone)]
pub struct RateLimitSnapshot {
    pub remaining: Option<u32>,
}

#[derive(Clone)]
pub struct OpenPrsClient {
    http: reqwest::Client,
}

impl Default for OpenPrsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenPrsClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn fetch_open_prs(
        &self,
        token: &str,
        repo_origins: &HashMap<String, String>,
    ) -> Result<(Vec<PullRequestPayload>, RateLimitSnapshot)> {
        let search = "is:pr is:open (author:@me OR review-requested:@me OR assignee:@me OR mentions:@me)";
        let body = serde_json::json!({
            "query": SEARCH_QUERY,
            "variables": { "q": search }
        });

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|error| Error::Github(error.to_string()))?,
        );
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("cormux"),
        );

        let response = self
            .http
            .post(GRAPHQL_URL)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|error| Error::Github(error.to_string()))?;

        let remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok());

        if response.status() == reqwest::StatusCode::FORBIDDEN && remaining == Some(0) {
            return Err(Error::Github("GitHub rate limit exceeded".into()));
        }

        let payload: GraphQlResponse = response
            .json()
            .await
            .map_err(|error| Error::Github(error.to_string()))?;

        if let Some(errors) = payload.errors {
            let message = errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Error::Github(message));
        }

        let data = payload
            .data
            .ok_or_else(|| Error::Github("empty GraphQL response".into()))?;
        let viewer = data.viewer.login;
        let mut items = Vec::new();
        let mut seen = HashMap::new();

        for node in data.search.nodes {
            let Some(pr) = node else { continue };
            let rel = infer_relationship(&pr, &viewer);
            let entry = map_pr(pr, &viewer, rel, repo_origins);
            let id = entry.cache_id();
            match seen.get(&id) {
                Some(existing_rel) if rel_priority(*existing_rel) >= rel_priority(rel) => continue,
                _ => {
                    seen.insert(id.clone(), rel);
                    if let Some(index) = items.iter().position(|row: &PullRequestPayload| row.cache_id() == id) {
                        items[index] = entry;
                    } else {
                        items.push(entry);
                    }
                }
            }
        }

        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok((items, RateLimitSnapshot { remaining }))
    }
}

fn rel_priority(rel: PrRelationship) -> u8 {
    match rel {
        PrRelationship::Review => 4,
        PrRelationship::Author => 3,
        PrRelationship::Assigned => 2,
        PrRelationship::Mention => 1,
    }
}

fn infer_relationship(pr: &PullRequestNode, viewer: &str) -> PrRelationship {
    let author = pr.author.login.as_str();
    if author.eq_ignore_ascii_case(viewer) {
        return PrRelationship::Author;
    }
    if pr.requested_reviewers.nodes.iter().any(|user| {
        user.login.eq_ignore_ascii_case(viewer)
    }) {
        return PrRelationship::Review;
    }
    if pr.assignees.nodes.iter().any(|user| {
        user.login.eq_ignore_ascii_case(viewer)
    }) {
        return PrRelationship::Assigned;
    }
    PrRelationship::Mention
}

fn map_pr(
    pr: PullRequestNode,
    viewer: &str,
    rel: PrRelationship,
    repo_origins: &HashMap<String, String>,
) -> PullRequestPayload {
    let repo_full_name = pr.repository.name_with_owner;
    let repo_id = super::r#match::match_repo_id(repo_origins, &repo_full_name);
    let author = if pr.author.login.eq_ignore_ascii_case(viewer) {
        "you".into()
    } else {
        pr.author.login
    };

    let (checks, failing) = map_checks(&pr.status_check_rollup.state);
    let review = map_review(pr.is_draft, pr.review_decision.as_deref());

    PullRequestPayload {
        num: pr.number,
        title: pr.title,
        author,
        rel,
        head: pr.head_ref_name,
        base: pr.base_ref_name,
        updated_at: pr.updated_at,
        checks,
        failing,
        review,
        additions: pr.additions,
        deletions: pr.deletions,
        files: pr.changed_files,
        is_draft: pr.is_draft,
        html_url: pr.url,
        repo_full_name,
        repo_id,
    }
}

fn map_checks(state: &str) -> (PrChecksState, Option<u32>) {
    match state {
        "SUCCESS" => (PrChecksState::Pass, None),
        "FAILURE" => (PrChecksState::Fail, Some(1)),
        "PENDING" | "EXPECTED" | "REQUESTED" => (PrChecksState::Running, None),
        _ => (PrChecksState::Running, None),
    }
}

fn map_review(is_draft: bool, decision: Option<&str>) -> PrReviewState {
    if is_draft {
        return PrReviewState::Draft;
    }
    match decision {
        Some("APPROVED") => PrReviewState::Approved,
        Some("CHANGES_REQUESTED") => PrReviewState::Changes,
        _ => PrReviewState::Required,
    }
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse {
    data: Option<GraphQlData>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphQlData {
    viewer: ViewerNode,
    search: SearchResults,
}

#[derive(Debug, Deserialize)]
struct ViewerNode {
    login: String,
}

#[derive(Debug, Deserialize)]
struct SearchResults {
    nodes: Vec<Option<PullRequestNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestNode {
    number: i64,
    title: String,
    is_draft: bool,
    url: String,
    updated_at: String,
    additions: i64,
    deletions: i64,
    changed_files: i64,
    review_decision: Option<String>,
    author: LoginNode,
    head_ref_name: String,
    base_ref_name: String,
    repository: RepositoryNode,
    status_check_rollup: StatusCheckRollup,
    requested_reviewers: UserConnection,
    assignees: UserConnection,
}

#[derive(Debug, Deserialize)]
struct LoginNode {
    login: String,
}

#[derive(Debug, Deserialize)]
struct RepositoryNode {
    name_with_owner: String,
}

#[derive(Debug, Deserialize)]
struct StatusCheckRollup {
    state: String,
}

#[derive(Debug, Deserialize)]
struct UserConnection {
    nodes: Vec<LoginNode>,
}
