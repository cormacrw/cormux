mod migrate;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{Error, Result};

/// SQLite persistence in the app data directory.
pub struct Store {
    conn: Mutex<Option<Connection>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            conn: Mutex::new(None),
        }
    }

    pub fn open(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate::run(&conn)?;

        *self
            .conn
            .lock()
            .map_err(|error| Error::Store(error.to_string()))? = Some(conn);

        log::info!("opened store at {}", path.display());
        Ok(())
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}
