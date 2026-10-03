//! Secrets Cormux keeps in `~/.cormux/credentials.json`, a flat JSON object of name → value.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub const CLICKUP_API_KEY: &str = "clickupApiKey";

fn path() -> io::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join(".cormux/credentials.json"))
}

pub fn get(name: &str) -> Option<String> {
    get_in(&path().ok()?, name)
}

pub fn set(name: &str, value: &str) -> io::Result<()> {
    update_in(&path()?, |all| {
        all.insert(name.to_string(), value.to_string());
    })
}

pub fn remove(name: &str) -> io::Result<()> {
    update_in(&path()?, |all| {
        all.remove(name);
    })
}

fn read_all(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn get_in(path: &Path, name: &str) -> Option<String> {
    read_all(path)
        .remove(name)
        .filter(|value| !value.is_empty())
}

fn update_in(path: &Path, change: impl FnOnce(&mut BTreeMap<String, String>)) -> io::Result<()> {
    let mut all = read_all(path);
    change(&mut all);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&all)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_gets_and_removes_without_touching_other_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".cormux/credentials.json");
        assert_eq!(get_in(&path, "a"), None);

        update_in(&path, |all| {
            all.insert("a".into(), "1".into());
            all.insert("b".into(), "2".into());
        })
        .unwrap();
        update_in(&path, |all| {
            all.remove("a");
        })
        .unwrap();

        assert_eq!(get_in(&path, "a"), None);
        assert_eq!(get_in(&path, "b").as_deref(), Some("2"));
    }
}
