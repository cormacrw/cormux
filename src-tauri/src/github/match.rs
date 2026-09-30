use std::collections::HashMap;

use crate::error::Result;
use crate::git::Git;
use crate::ipc::commands::expand_tilde;

/// Normalised `owner/repo` slug used to match GitHub PRs to registered repos.
pub fn parse_origin_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        return normalise_slug(strip_git_suffix(rest));
    }
    if let Some(rest) = trimmed.strip_prefix("ssh://git@github.com/") {
        return normalise_slug(strip_git_suffix(rest));
    }

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);

    let path = without_scheme
        .strip_prefix("github.com/")
        .or_else(|| without_scheme.strip_prefix("www.github.com/"))?;

    normalise_slug(strip_git_suffix(path))
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

pub fn match_repo_id(
    repo_origins: &HashMap<String, String>,
    pr_repo: &str,
) -> Option<String> {
    let needle = pr_repo.to_ascii_lowercase();
    repo_origins
        .iter()
        .find(|(_, slug)| slug.to_ascii_lowercase() == needle)
        .map(|(id, _)| id.clone())
}

pub async fn load_repo_origins(git: &Git, repos: &[(String, String)]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for (repo_id, path) in repos {
        let url = git.remote_origin_url(&expand_tilde(path)).await?;
        if let Some(url) = url {
            if let Some(slug) = parse_origin_url(&url) {
                map.insert(repo_id.clone(), slug);
            }
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
