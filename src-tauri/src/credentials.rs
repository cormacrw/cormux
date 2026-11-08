//! Secrets Cormux keeps in `~/.cormux/credentials.json`, a flat JSON object of name → value.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub const CLICKUP_API_KEY: &str = "clickupApiKey";

fn path() -> io::Result<PathBuf> {
    crate::paths::credentials()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))
}

/// The secret, or `None` when it isn't saved or the file can't be read (which is logged).
pub fn get(name: &str) -> Option<String> {
    match get_in(&path().ok()?, name) {
        Ok(value) => value,
        Err(error) => {
            log::warn!("reading {name}: {error}");
            None
        }
    }
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

/// Every saved secret; none when there's no file yet. A file that's there but isn't a JSON
/// object of strings is an error, so a write never replaces secrets it couldn't read.
fn read_all(path: &Path) -> io::Result<BTreeMap<String, String>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error),
    };
    serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} isn't a JSON object of names to values ({error}); fix or remove it first",
                crate::paths::for_display(path)
            ),
        )
    })
}

fn get_in(path: &Path, name: &str) -> io::Result<Option<String>> {
    Ok(read_all(path)?
        .remove(name)
        .filter(|value| !value.is_empty()))
}

fn update_in(path: &Path, change: impl FnOnce(&mut BTreeMap<String, String>)) -> io::Result<()> {
    let mut all = read_all(path)?;
    change(&mut all);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Created owner-only from the start, so the secret is never readable while being written.
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    io::Write::write_all(&mut file, serde_json::to_string_pretty(&all)?.as_bytes())?;
    #[cfg(unix)]
    {
        // An existing file keeps its old mode on open; make sure it ends up owner-only too.
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
        assert_eq!(get_in(&path, "a").unwrap(), None);

        update_in(&path, |all| {
            all.insert("a".into(), "1".into());
            all.insert("b".into(), "2".into());
        })
        .unwrap();
        update_in(&path, |all| {
            all.remove("a");
        })
        .unwrap();

        assert_eq!(get_in(&path, "a").unwrap(), None);
        assert_eq!(get_in(&path, "b").unwrap().as_deref(), Some("2"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn a_file_it_cant_read_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");
        let broken = "{\"githubToken\": \"keep me\",}";
        std::fs::write(&path, broken).unwrap();

        let error = update_in(&path, |all| {
            all.insert("clickupApiKey".into(), "new".into());
        })
        .unwrap_err();

        assert!(error.to_string().contains("fix or remove it"), "{error}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
        assert!(get_in(&path, "githubToken").is_err());
    }
}
