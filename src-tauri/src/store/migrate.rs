use rusqlite::Connection;

use crate::error::{Error, Result};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "001_initial",
        sql: include_str!("../../migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "002_sessions_findings_usage",
        sql: include_str!("../../migrations/002_sessions_findings_usage.sql"),
    },
    Migration {
        version: 3,
        name: "003_workspace_card_fields",
        sql: include_str!("../../migrations/003_workspace_card_fields.sql"),
    },
    Migration {
        version: 4,
        name: "004_workspace_archive",
        sql: include_str!("../../migrations/004_workspace_archive.sql"),
    },
    Migration {
        version: 5,
        name: "005_workspace_pr_url",
        sql: include_str!("../../migrations/005_workspace_pr_url.sql"),
    },
    Migration {
        version: 6,
        name: "006_findings_sent_thread",
        sql: include_str!("../../migrations/006_findings_sent_thread.sql"),
    },
    Migration {
        version: 7,
        name: "007_scratches",
        sql: include_str!("../../migrations/007_scratches.sql"),
    },
    Migration {
        version: 8,
        name: "008_thread_closed",
        sql: include_str!("../../migrations/008_thread_closed.sql"),
    },
];

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    // Table rebuilds (007) drop a parent table, which would cascade-delete its children while
    // foreign keys are on. The pragma is ignored inside a transaction, so set it out here.
    conn.pragma_update(None, "foreign_keys", false)?;
    let result = apply_pending(conn);
    conn.pragma_update(None, "foreign_keys", true)?;
    result
}

fn apply_pending(conn: &Connection) -> Result<()> {
    for migration in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [migration.version],
            |row| row.get(0),
        )?;

        if already_applied {
            continue;
        }

        conn.execute_batch("BEGIN IMMEDIATE;")?;
        if let Err(error) = apply(conn, migration) {
            conn.execute_batch("ROLLBACK;").ok();
            return Err(error);
        }
        conn.execute_batch("COMMIT;")?;
        log::info!("applied migration {}", migration.name);
    }

    let broken: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_foreign_key_check",
        [],
        |row| row.get(0),
    )?;
    if broken > 0 {
        log::warn!("{broken} rows fail foreign key checks after migrating");
    }
    Ok(())
}

fn apply(conn: &Connection, migration: &Migration) -> Result<()> {
    conn.execute_batch(migration.sql)
        .map_err(|error| Error::Store(format!("{}: {error}", migration.name)))?;
    conn.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        rusqlite::params![migration.version, migration.name],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_initial_schema() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        run(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|name| name.unwrap())
            .collect();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 8);

        for expected in [
            "approvals",
            "findings",
            "pr_cache",
            "repo_config",
            "repos",
            "schema_migrations",
            "scratches",
            "settings",
            "thread_events",
            "threads",
            "workspaces",
        ] {
            assert!(
                tables.iter().any(|name| name == expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn scratches_migration_keeps_thread_history() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
        for migration in &MIGRATIONS[..6] {
            apply(&conn, migration).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO repos (id, path, name) VALUES ('r', '/tmp/r', 'r');
             INSERT INTO workspaces (id, repo_id, name, branch, worktree_path, status)
                VALUES ('w', 'r', 'W', 'b', '/tmp/w', 'idle');
             INSERT INTO threads (id, workspace_id, title, engine, status, used_tokens)
                VALUES ('t', 'w', 'Lead', 'claude', 'idle', 42);
             INSERT INTO thread_events (thread_id, seq, kind, payload) VALUES ('t', 1, 'message', '{}');
             INSERT INTO approvals (id, thread_id, status, tool, payload)
                VALUES ('a', 't', 'pending', 'Edit', '{}');",
        )
        .unwrap();

        run(&conn).unwrap();

        let events: i64 = conn
            .query_row("SELECT COUNT(*) FROM thread_events", [], |row| row.get(0))
            .unwrap();
        let approvals: i64 = conn
            .query_row("SELECT COUNT(*) FROM approvals", [], |row| row.get(0))
            .unwrap();
        let tokens: i64 = conn
            .query_row("SELECT used_tokens FROM threads WHERE id = 't'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!((events, approvals, tokens), (1, 1, 42));

        // Scratch threads have no workspace row.
        conn.execute(
            "INSERT INTO threads (id, workspace_id, title, engine, status)
             VALUES ('s', 'scratch-1', 'Scratch', 'claude', 'idle')",
            [],
        )
        .unwrap();
        // Child rows still cascade with their thread.
        conn.execute("DELETE FROM threads WHERE id = 't'", []).unwrap();
        let events: i64 = conn
            .query_row("SELECT COUNT(*) FROM thread_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(events, 0);
    }
}
