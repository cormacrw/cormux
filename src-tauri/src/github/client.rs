use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::RwLock;

use crate::error::Error;
use crate::shell_env::ShellEnv;

use super::types::{
    PrChecksState, PrRelationship, PrReviewState, PullRequestPayload,
};

const SEARCH_QUERY: &str = r#"
query($q: String!, $after: String) {
  viewer { login }
  search(query: $q, type: ISSUE_ADVANCED, first: 100, after: $after) {
    pageInfo { hasNextPage endCursor }
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
        reviewRequests(first: 20) {
          nodes { requestedReviewer { ... on User { login } } }
        }
        assignees(first: 20) {
          nodes { login }
        }
      }
    }
  }
}
"#;

/// Parenthesised `OR` needs `ISSUE_ADVANCED`; the legacy `ISSUE` type silently returns nothing.
const SEARCH: &str = "is:pr is:open (author:@me OR review-requested:@me OR assignee:@me OR mentions:@me)";

/// Hard stop so a runaway search can't burn the rate limit.
const MAX_PAGES: usize = 5;

/// How a fetch failed, so callers only drop the cache when the user is really signed out.
#[derive(Debug)]
pub enum FetchError {
    SignedOut(String),
    Other(Error),
}

impl From<FetchError> for Error {
    fn from(error: FetchError) -> Self {
        match error {
            FetchError::SignedOut(message) => Error::Github(message),
            FetchError::Other(error) => error,
        }
    }
}

/// `gh` prints these when there is no usable token, as opposed to a network or API hiccup.
fn is_signed_out(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("gh auth login")
        || lower.contains("not logged into")
        || lower.contains("bad credentials")
        || lower.contains("http 401")
}

pub async fn fetch_open_prs(
    env: &Arc<RwLock<ShellEnv>>,
    repo_origins: &HashMap<String, String>,
) -> std::result::Result<Vec<PullRequestPayload>, FetchError> {
    let query_arg = format!("query={SEARCH_QUERY}");
    let search_arg = format!("q={SEARCH}");
    let mut viewer = String::new();
    let mut nodes = Vec::new();
    let mut after: Option<String> = None;

    for _ in 0..MAX_PAGES {
        let after_arg = after.as_ref().map(|cursor| format!("after={cursor}"));
        let mut args = vec!["api", "graphql", "-f", &query_arg, "-f", &search_arg];
        if let Some(after_arg) = &after_arg {
            args.extend(["-f", after_arg]);
        }
        let output = {
            let shell = env.read().await;
            shell.run("gh", &args, None).await
        };
        let output = output.map_err(|error| {
            let message = error.to_string();
            if is_signed_out(&message) {
                FetchError::SignedOut(message)
            } else {
                FetchError::Other(Error::Github(message))
            }
        })?;

        let payload: GraphQlResponse = serde_json::from_str(&output).map_err(|error| {
            FetchError::Other(Error::Github(format!("gh api graphql: {error}")))
        })?;

        if let Some(errors) = payload.errors {
            let message = errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(FetchError::Other(Error::Github(message)));
        }

        let data = payload
            .data
            .ok_or_else(|| FetchError::Other(Error::Github("empty GraphQL response".into())))?;
        viewer = data.viewer.login;
        nodes.extend(data.search.nodes.into_iter().flatten());
        match data.search.page_info {
            PageInfo {
                has_next_page: true,
                end_cursor: Some(cursor),
            } => after = Some(cursor),
            _ => break,
        }
    }

    let mut items: Vec<PullRequestPayload> = Vec::new();
    let mut seen: HashMap<String, (usize, PrRelationship)> = HashMap::new();
    for pr in nodes {
        let rel = infer_relationship(&pr, &viewer);
        let entry = map_pr(pr, &viewer, rel, repo_origins);
        let id = entry.cache_id();
        match seen.get(&id) {
            Some((_, existing)) if rel_priority(*existing) >= rel_priority(rel) => {}
            Some((index, _)) => {
                let index = *index;
                items[index] = entry;
                seen.insert(id, (index, rel));
            }
            None => {
                seen.insert(id, (items.len(), rel));
                items.push(entry);
            }
        }
    }

    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(items)
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
    let author = pr.author.as_ref().map_or("", |author| author.login.as_str());
    if author.eq_ignore_ascii_case(viewer) {
        return PrRelationship::Author;
    }
    if pr.review_requests.nodes.iter().flatten().any(|request| {
        request
            .requested_reviewer
            .as_ref()
            .and_then(|reviewer| reviewer.login.as_deref())
            .is_some_and(|login| login.eq_ignore_ascii_case(viewer))
    }) {
        return PrRelationship::Review;
    }
    if pr.assignees.nodes.iter().flatten().any(|user| {
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
    // Deleted accounts come back as a null author; GitHub shows them as "ghost".
    let author = match pr.author {
        Some(author) if author.login.eq_ignore_ascii_case(viewer) => "you".into(),
        Some(author) => author.login,
        None => "ghost".into(),
    };

    let (checks, failing) = map_checks(
        pr.status_check_rollup
            .as_ref()
            .and_then(|rollup| rollup.state.as_deref())
            .unwrap_or(""),
    );
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
        "FAILURE" | "ERROR" => (PrChecksState::Fail, Some(1)),
        // A null rollup means no CI is configured, not that it's still running.
        "" => (PrChecksState::None, None),
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
#[serde(rename_all = "camelCase")]
struct SearchResults {
    page_info: PageInfo,
    nodes: Vec<Option<PullRequestNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
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
    author: Option<LoginNode>,
    head_ref_name: String,
    base_ref_name: String,
    repository: RepositoryNode,
    status_check_rollup: Option<StatusCheckRollup>,
    review_requests: ReviewRequestConnection,
    assignees: UserConnection,
}

#[derive(Debug, Deserialize)]
struct LoginNode {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryNode {
    name_with_owner: String,
}

#[derive(Debug, Deserialize)]
struct StatusCheckRollup {
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReviewRequestConnection {
    nodes: Vec<Option<ReviewRequestNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewRequestNode {
    requested_reviewer: Option<ReviewerNode>,
}

/// Teams, bots and mannequins can be requested too; only users carry a login here.
#[derive(Debug, Deserialize)]
struct ReviewerNode {
    login: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserConnection {
    nodes: Vec<Option<LoginNode>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"{
      "data": {
        "viewer": { "login": "cormacrw" },
        "search": {
          "pageInfo": { "hasNextPage": false, "endCursor": "Y3Vyc29yOjM=" },
          "nodes": [
            {
              "number": 2, "title": "Test Feature", "isDraft": false,
              "url": "https://github.com/cormacrw/todo-example/pull/2",
              "updatedAt": "2026-09-30T00:52:31Z",
              "additions": 56, "deletions": 7, "changedFiles": 6,
              "reviewDecision": null,
              "author": { "login": "cormacrw" },
              "headRefName": "feat/test", "baseRefName": "main",
              "repository": { "nameWithOwner": "cormacrw/todo-example" },
              "statusCheckRollup": null,
              "reviewRequests": { "nodes": [] },
              "assignees": { "nodes": [] }
            },
            {
              "number": 9, "title": "From a deleted account", "isDraft": true,
              "url": "https://github.com/acme/app/pull/9",
              "updatedAt": "2026-09-29T00:00:00Z",
              "additions": 1, "deletions": 0, "changedFiles": 1,
              "reviewDecision": "APPROVED",
              "author": null,
              "headRefName": "x", "baseRefName": "main",
              "repository": { "nameWithOwner": "acme/app" },
              "statusCheckRollup": { "state": "SUCCESS" },
              "reviewRequests": { "nodes": [
                { "requestedReviewer": {} },
                { "requestedReviewer": { "login": "cormacrw" } }
              ] },
              "assignees": { "nodes": [] }
            },
            null
          ]
        }
      }
    }"#;

    #[test]
    fn parses_live_search_response() {
        let payload: GraphQlResponse = serde_json::from_str(RESPONSE).unwrap();
        let data = payload.data.unwrap();
        assert_eq!(data.search.nodes.len(), 3);
        let viewer = data.viewer.login;
        let prs: Vec<_> = data
            .search
            .nodes
            .into_iter()
            .flatten()
            .map(|pr| {
                let rel = infer_relationship(&pr, &viewer);
                map_pr(pr, &viewer, rel, &HashMap::new())
            })
            .collect();
        assert_eq!(prs[0].repo_full_name, "cormacrw/todo-example");
        assert_eq!(prs[0].author, "you");
        assert!(matches!(prs[0].rel, PrRelationship::Author));
        assert!(matches!(prs[0].checks, PrChecksState::None));
        assert!(matches!(prs[1].checks, PrChecksState::Pass));
        assert_eq!(prs[1].author, "ghost");
        assert!(matches!(prs[1].rel, PrRelationship::Review));
    }

    #[test]
    fn signed_out_only_for_auth_failures() {
        assert!(is_signed_out("gh failed: To get started with GitHub CLI, please run:  gh auth login"));
        assert!(is_signed_out("gh failed: HTTP 401: Bad credentials (https://api.github.com/graphql)"));
        assert!(!is_signed_out("gh failed: error connecting to api.github.com"));
        assert!(!is_signed_out("gh: No such file or directory (os error 2)"));
    }
}
