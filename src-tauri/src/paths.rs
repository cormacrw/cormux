//! Where things live on disk.

use std::path::PathBuf;

use crate::error::Result;
use crate::store::Store;
use crate::workspace::WorkspaceManager;

pub fn harness_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn worktrees_base(store: &Store) -> Result<PathBuf> {
    Ok(store
        .get_setting("worktreeRoot")?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| expand_tilde(&value))
        .unwrap_or_else(|| WorkspaceManager::default_worktrees_base(&harness_home())))
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_leading_tilde_slash_expands() {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap();
        assert_eq!(expand_tilde("~/code/app"), home.join("code/app"));
        assert_eq!(expand_tilde("/code/~/app"), PathBuf::from("/code/~/app"));
        assert_eq!(expand_tilde("~other/app"), PathBuf::from("~other/app"));
    }

    #[test]
    fn the_worktree_root_setting_overrides_the_default() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        let default = WorkspaceManager::default_worktrees_base(&harness_home());
        assert_eq!(worktrees_base(&store).unwrap(), default);

        store.set_setting("worktreeRoot", "   ").unwrap();
        assert_eq!(worktrees_base(&store).unwrap(), default);

        store.set_setting("worktreeRoot", " /srv/trees ").unwrap();
        assert_eq!(worktrees_base(&store).unwrap(), PathBuf::from("/srv/trees"));
    }
}
