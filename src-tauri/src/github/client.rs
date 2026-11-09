use std::collections::HashMap;

use serde::Deserialize;

use crate::error::Error;

use super::Gh;

use super::types::{PrChecksState, PrReviewState, PullRequestPayload, VIEWER_AUTHOR};

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
        changedFiles
        reviewDecision
        author { login }
        headRefName
        baseRefName
        repository { nameWithOwner }
        statusCheckRollup { state }
      }
    }
  }
}
"#;

/// Parenthesised `OR` needs `ISSUE_ADVANCED`; the legacy `ISSUE` type silently returns nothing.
const SEARCH: &str =
    "is:pr is:open (author:@me OR review-requested:@me OR assignee:@me OR mentions:@me)";

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
    gh: &Gh,
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
        let output = gh
            .output(None, &args, &[])
            .await
            .map_err(|failure| FetchError::Other(failure.into()))?;
        let data = read_page(output.status.success(), &output.stdout, &output.stderr)?;
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

    // A PR that matched the search more than once (authored and mentioned) is listed once.
    let mut seen = std::collections::HashSet::new();
    let mut items: Vec<PullRequestPayload> = nodes
        .into_iter()
        .map(|pr| map_pr(pr, &viewer, repo_origins))
        .filter(|pr| seen.insert(pr.cache_id()))
        .collect();

    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(items)
}

/// One page of the search from `gh api graphql`'s output. GitHub returns partial data plus
/// errors (e.g. SAML-protected orgs the token can't see), and `gh` exits non-zero for it, so
/// the body is read whatever the exit status: what came back is kept rather than dropping the
/// sync.
fn read_page(
    succeeded: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> std::result::Result<GraphQlData, FetchError> {
    let parsed = serde_json::from_slice::<GraphQlResponse>(stdout);
    let payload = match parsed {
        Ok(payload) if succeeded || payload.data.is_some() => payload,
        Err(error) if succeeded => {
            return Err(FetchError::Other(Error::Github(format!(
                "gh api graphql: {error}"
            ))));
        }
        _ => {
            let message = format!("gh api failed: {}", String::from_utf8_lossy(stderr).trim());
            return Err(if is_signed_out(&message) {
                FetchError::SignedOut(message)
            } else {
                FetchError::Other(Error::Github(message))
            });
        }
    };

    if let Some(errors) = payload.errors {
        let message = errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; ");
        if payload.data.is_none() {
            return Err(FetchError::Other(Error::Github(message)));
        }
        log::warn!("gh api graphql returned partial results: {message}");
    }

    payload
        .data
        .ok_or_else(|| FetchError::Other(Error::Github("empty GraphQL response".into())))
}

fn map_pr(
    pr: PullRequestNode,
    viewer: &str,
    repo_origins: &HashMap<String, String>,
) -> PullRequestPayload {
    let repo_full_name = pr.repository.name_with_owner;
    let repo_id = super::r#match::match_repo_id(repo_origins, &repo_full_name);
    // Deleted accounts come back as a null author; GitHub shows them as "ghost".
    let author = match pr.author {
        Some(author) if author.login.eq_ignore_ascii_case(viewer) => VIEWER_AUTHOR.into(),
        Some(author) => author.login,
        None => "ghost".into(),
    };

    let checks = map_checks(
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
        head: pr.head_ref_name,
        base: pr.base_ref_name,
        updated_at: pr.updated_at,
        checks,
        review,
        files: pr.changed_files,
        is_draft: pr.is_draft,
        html_url: pr.url,
        repo_full_name,
        repo_id,
    }
}

fn map_checks(state: &str) -> PrChecksState {
    match state {
        "SUCCESS" => PrChecksState::Pass,
        "FAILURE" | "ERROR" => PrChecksState::Fail,
        // A null rollup means no CI is configured, not that it's still running.
        "" => PrChecksState::None,
        _ => PrChecksState::Running,
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
    changed_files: i64,
    review_decision: Option<String>,
    author: Option<LoginNode>,
    head_ref_name: String,
    base_ref_name: String,
    repository: RepositoryNode,
    status_check_rollup: Option<StatusCheckRollup>,
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
              "changedFiles": 6,
              "reviewDecision": null,
              "author": { "login": "cormacrw" },
              "headRefName": "feat/test", "baseRefName": "main",
              "repository": { "nameWithOwner": "cormacrw/todo-example" },
              "statusCheckRollup": null
            },
            {
              "number": 9, "title": "From a deleted account", "isDraft": true,
              "url": "https://github.com/acme/app/pull/9",
              "updatedAt": "2026-09-29T00:00:00Z",
              "changedFiles": 1,
              "reviewDecision": "APPROVED",
              "author": null,
              "headRefName": "x", "baseRefName": "main",
              "repository": { "nameWithOwner": "acme/app" },
              "statusCheckRollup": { "state": "SUCCESS" }
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
            .map(|pr| map_pr(pr, &viewer, &HashMap::new()))
            .collect();
        assert_eq!(prs[0].repo_full_name, "cormacrw/todo-example");
        assert_eq!(prs[0].author, "you");
        assert!(matches!(prs[0].checks, PrChecksState::None));
        assert!(matches!(prs[1].checks, PrChecksState::Pass));
        assert_eq!(prs[1].author, "ghost");
    }

    /// Bug: `gh` exits 1 when the response has errors beside data, and that exit was treated
    /// as a failed sync, so one SAML-protected org stopped every PR from syncing.
    #[test]
    fn partial_results_are_kept_whatever_gh_exits_with() {
        let partial = RESPONSE.replacen(
            "{",
            r#"{ "errors": [{ "message": "Resource protected by organization SAML enforcement" }],"#,
            1,
        );
        let data = read_page(
            false,
            partial.as_bytes(),
            b"gh: GraphQL: Resource protected",
        )
        .unwrap();
        assert_eq!(data.search.nodes.len(), 3);

        let only_errors = r#"{ "errors": [{ "message": "Something went wrong" }] }"#;
        assert!(matches!(
            read_page(
                false,
                only_errors.as_bytes(),
                b"gh: GraphQL: Something went wrong"
            ),
            Err(FetchError::Other(_))
        ));
        assert!(matches!(
            read_page(false, b"", b"HTTP 401: Bad credentials"),
            Err(FetchError::SignedOut(_))
        ));
    }

    #[test]
    fn signed_out_only_for_auth_failures() {
        assert!(is_signed_out(
            "gh failed: To get started with GitHub CLI, please run:  gh auth login"
        ));
        assert!(is_signed_out(
            "gh failed: HTTP 401: Bad credentials (https://api.github.com/graphql)"
        ));
        assert!(!is_signed_out(
            "gh failed: error connecting to api.github.com"
        ));
        assert!(!is_signed_out("gh: No such file or directory (os error 2)"));
    }
}
