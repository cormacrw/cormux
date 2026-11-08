use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::git::Git;
use crate::paths::expand_tilde;

/// A GitHub repository, as the REST API addresses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSlug {
    pub owner: String,
    pub name: String,
}

impl RepoSlug {
    /// From a GitHub remote URL or any github.com page of the repo (a pull request's).
    pub fn parse(url: &str) -> Option<Self> {
        let slug = parse_origin_url(url)?;
        let (owner, name) = slug.split_once('/')?;
        Some(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    /// `owner/name`.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    pub async fn of_origin(git: &Git, path: &Path) -> Result<Self> {
        let origin = git
            .remote_origin_url(path)
            .await?
            .ok_or_else(|| Error::Git("no origin remote configured for this repo".into()))?;
        Self::parse(&origin).ok_or_else(|| {
            Error::Git(format!(
                "could not parse origin URL {}",
                redact_origin_url(&origin)
            ))
        })
    }
}

/// Normalised `owner/repo` slug used to match GitHub PRs to registered repos.
pub fn parse_origin_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // URL form (`scheme://[user[:token]@]host[:port]/owner/repo`) or bare `host/owner/repo`.
    let (authority, path) = if let Some((_, rest)) = trimmed.split_once("://") {
        rest.split_once('/')?
    } else if let Some((host, rest)) = scp_parts(trimmed) {
        // scp-like SSH form: `[user@]host:owner/repo`.
        (host, rest)
    } else {
        trimmed.split_once('/')?
    };

    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = host.split_once(':').map_or(host, |(host, _port)| host);
    if !host.eq_ignore_ascii_case("github.com") && !host.eq_ignore_ascii_case("www.github.com") {
        return None;
    }

    normalise_slug(strip_git_suffix(path))
}

/// Splits `[user@]host:path` when the colon comes before any slash.
fn scp_parts(raw: &str) -> Option<(&str, &str)> {
    let (host, path) = raw.split_once(':')?;
    (!host.contains('/')).then_some((host, path))
}

/// The origin URL with any embedded credentials removed, safe to show in errors.
pub fn redact_origin_url(raw: &str) -> String {
    match raw.split_once("://") {
        Some((scheme, rest)) => {
            let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
            match authority.rsplit_once('@') {
                Some((_, host)) => format!("{scheme}://***@{host}/{path}"),
                None => raw.to_string(),
            }
        }
        None => raw.to_string(),
    }
}

fn strip_git_suffix(path: &str) -> &str {
    path.trim_end_matches(".git").trim_matches('/')
}

fn normalise_slug(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0].to_ascii_lowercase();
    let repo = parts[1].to_string();
    Some(format!("{owner}/{repo}"))
}

pub fn match_repo_id(repo_origins: &HashMap<String, String>, pr_repo: &str) -> Option<String> {
    let needle = pr_repo.to_ascii_lowercase();
    repo_origins
        .iter()
        .find(|(_, slug)| slug.to_ascii_lowercase() == needle)
        .map(|(id, _)| id.clone())
}

pub async fn load_repo_origins(
    git: &Git,
    repos: &[(String, String)],
) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for (repo_id, path) in repos {
        // A moved or deleted repo shouldn't stop PR sync for every other repo.
        let url = match git.remote_origin_url(&expand_tilde(path)).await {
            Ok(url) => url,
            Err(error) => {
                log::warn!("skipping repo {repo_id} for PR sync: {error}");
                continue;
            }
        };
        if let Some(url) = url
            && let Some(slug) = parse_origin_url(&url)
        {
            map.insert(repo_id.clone(), slug);
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_origin() {
        assert_eq!(
            parse_origin_url("https://github.com/cormacrw/cormux.git"),
            Some("cormacrw/cormux".into())
        );
    }

    #[test]
    fn parse_ssh_origin() {
        assert_eq!(
            parse_origin_url("git@github.com:Acme/My-App.git"),
            Some("acme/My-App".into())
        );
    }

    #[test]
    fn parse_ssh_url_scheme() {
        assert_eq!(
            parse_origin_url("ssh://git@github.com/org/repo"),
            Some("org/repo".into())
        );
    }

    #[test]
    fn parse_origin_with_credentials_and_port() {
        assert_eq!(
            parse_origin_url("https://x-access-token:ghp_secret@github.com/o/r.git"),
            Some("o/r".into())
        );
        assert_eq!(
            parse_origin_url("ssh://git@github.com:22/o/r.git"),
            Some("o/r".into())
        );
        assert_eq!(parse_origin_url("github.com/o/r"), Some("o/r".into()));
        assert_eq!(parse_origin_url("git@gitlab.com:o/r.git"), None);
    }

    #[test]
    fn a_slug_comes_from_a_remote_or_a_pull_request_page() {
        let slug = |owner: &str, name: &str| RepoSlug {
            owner: owner.into(),
            name: name.into(),
        };
        assert_eq!(
            RepoSlug::parse("https://github.com/Acme/app/pull/12"),
            Some(slug("acme", "app"))
        );
        assert_eq!(
            RepoSlug::parse("git@github.com:o/r.git"),
            Some(slug("o", "r"))
        );
        assert_eq!(RepoSlug::parse("https://gitlab.com/o/r/pull/1"), None);
    }

    #[test]
    fn redact_strips_userinfo() {
        assert_eq!(
            redact_origin_url("https://x-access-token:ghp_secret@github.com/o/r.git"),
            "https://***@github.com/o/r.git"
        );
        assert_eq!(
            redact_origin_url("git@github.com:o/r.git"),
            "git@github.com:o/r.git"
        );
    }

    #[test]
    fn match_finds_registered_repo() {
        let mut origins = HashMap::new();
        origins.insert("r1".into(), "cormacrw/cormux".into());
        assert_eq!(
            match_repo_id(&origins, "CormacRW/Cormux"),
            Some("r1".into())
        );
        assert_eq!(match_repo_id(&origins, "other/repo"), None);
    }
}
