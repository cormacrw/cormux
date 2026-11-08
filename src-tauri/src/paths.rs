//! Where things live on disk. `HOME` is read here and nowhere else.

use std::path::{Path, PathBuf};

pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = home()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// The path as the user would type it: resolved, with the home folder shown as `~`.
pub fn for_display(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if let Some(home) = home()
        && let Ok(rest) = canonical.strip_prefix(&home)
    {
        return format!("~/{}", rest.to_string_lossy());
    }
    canonical.to_string_lossy().to_string()
}

/// Secrets Cormux keeps outside the database. `None` without a home folder.
pub fn credentials() -> Option<PathBuf> {
    home().map(|home| home.join(".cormux/credentials.json"))
}

/// The settings a repo can commit for Cormux.
pub fn repo_config(repo: &Path) -> PathBuf {
    repo.join(".cormux/config.json")
}

/// The folder new worktrees go in, from the setting as the user typed it.
pub fn worktrees_root(setting: &str) -> PathBuf {
    expand_tilde(setting.trim())
}

/// Where a workspace for `branch` of the repo at `repo` checks out.
pub fn worktree_path(root: &Path, repo: &Path, branch: &str) -> PathBuf {
    root.join(repo_dir_name(repo)).join(branch_slug(branch))
}

/// A branch's folder name. `feat/x` and `feat-x` share one.
pub fn branch_slug(branch: &str) -> String {
    branch.replace('/', "-")
}

pub fn repo_dir_name(repo: &Path) -> &str {
    repo.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
}

/// The app's own data folder and what it keeps there.
#[derive(Debug, Clone)]
pub struct DataDir(PathBuf);

impl DataDir {
    #[cfg(test)]
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Inside the OS's app data folder. Dev builds keep their own, so running Cormux from
    /// inside Cormux can't touch the installed app's workspaces.
    pub fn for_app(app_data: PathBuf) -> Self {
        if cfg!(debug_assertions) {
            Self(app_data.join("dev"))
        } else {
            Self(app_data)
        }
    }

    pub fn db(&self) -> PathBuf {
        self.0.join("cormux.db")
    }

    /// Images sent with prompts.
    pub fn attachments(&self) -> PathBuf {
        self.0.join("attachments")
    }

    /// Copies of media agents referenced, which the webview may load.
    pub fn media(&self) -> PathBuf {
        self.0.join("media")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_leading_tilde_slash_expands() {
        let home = home().unwrap();
        assert_eq!(expand_tilde("~/code/app"), home.join("code/app"));
        assert_eq!(expand_tilde("/code/~/app"), PathBuf::from("/code/~/app"));
        assert_eq!(expand_tilde("~other/app"), PathBuf::from("~other/app"));
    }

    #[test]
    fn the_worktree_root_setting_is_read_as_typed() {
        assert_eq!(
            worktrees_root("~/.harness/worktrees"),
            home().unwrap().join(".harness/worktrees")
        );
        assert_eq!(worktrees_root(" /srv/trees "), PathBuf::from("/srv/trees"));
    }

    #[test]
    fn worktree_path_includes_repo_and_branch_slug() {
        assert_eq!(
            worktree_path(Path::new("/trees"), Path::new("/code/my-app"), "feat/login"),
            PathBuf::from("/trees/my-app/feat-login")
        );
    }
}
