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
    Migration {
        version: 9,
        name: "009_todos",
        sql: include_str!("../../migrations/009_todos.sql"),
    },
    Migration {
        version: 10,
        name: "010_thread_model",
        sql: include_str!("../../migrations/010_thread_model.sql"),
    },
    Migration {
        version: 11,
        name: "011_repo_single_instance",
        sql: include_str!("../../migrations/011_repo_single_instance.sql"),
    },
    Migration {
        version: 12,
        name: "012_week_plan",
        sql: include_str!("../../migrations/012_week_plan.sql"),
    },
    Migration {
        version: 13,
        name: "013_todo_number",
        sql: include_str!("../../migrations/013_todo_number.sql"),
    },
    Migration {
        version: 14,
        name: "014_drop_cursor",
        sql: include_str!("../../migrations/014_drop_cursor.sql"),
    },
    Migration {
        version: 15,
        name: "015_one_thread_per_workspace",
        sql: include_str!("../../migrations/015_one_thread_per_workspace.sql"),
    },
    Migration {
        version: 16,
        name: "016_drop_plan_mode",
        sql: include_str!("../../migrations/016_drop_plan_mode.sql"),
    },
    Migration {
        version: 17,
        name: "017_thread_parent",
        sql: include_str!("../../migrations/017_thread_parent.sql"),
    },
    Migration {
        version: 18,
        name: "018_read_path_indexes",
        sql: include_str!("../../migrations/018_read_path_indexes.sql"),
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

    let broken: i64 =
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
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
        assert_eq!(version, 18);

        for expected in [
            "approvals",
            "findings",
            "pr_cache",
            "repo_config",
            "repos",
            "schema_migrations",
            "scratches",
            "settings",
            "sprint_plan",
            "thread_events",
            "threads",
            "todos",
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
            .query_row(
                "SELECT used_tokens FROM threads WHERE id = 't'",
                [],
                |row| row.get(0),
            )
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
        conn.execute("DELETE FROM threads WHERE id = 't'", [])
            .unwrap();
        let events: i64 = conn
            .query_row("SELECT COUNT(*) FROM thread_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(events, 0);
    }

    #[test]
    fn one_thread_stays_live_per_workspace() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
        for migration in &MIGRATIONS[..14] {
            apply(&conn, migration).unwrap();
        }
        // `lead` was created first, but `helper` spoke last, so `helper` carries on.
        conn.execute_batch(
            "INSERT INTO threads (id, workspace_id, title, engine, status) VALUES
                ('lead', 'w', 'Lead', 'claude', 'waiting'),
                ('helper', 'w', 'Agent 2', 'claude', 'idle'),
                ('quiet', 'w', 'Agent 3', 'claude', 'idle'),
                ('solo', 'w2', 'Lead', 'claude', 'idle'),
                ('scratch', 's', 'Scratch', 'claude', 'idle');
             INSERT INTO scratches (id, repo_id, title, thread_id)
                VALUES ('s', 'r', 'A question', 'scratch');
             INSERT INTO thread_events (thread_id, seq, kind, payload) VALUES
                ('lead', 1, 'message', '{}'),
                ('helper', 1, 'message', '{}');
             INSERT INTO approvals (id, thread_id, status, tool, payload)
                VALUES ('a', 'lead', 'pending', 'bash', '{}');",
        )
        .unwrap();

        run(&conn).unwrap();

        let retired = |id: &str| -> bool {
            conn.query_row("SELECT retired FROM threads WHERE id = ?1", [id], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap()
                != 0
        };
        assert!(retired("lead"));
        assert!(!retired("helper"));
        assert!(retired("quiet"));
        assert!(!retired("solo"));
        assert!(retired("scratch"));
        let approval: String = conn
            .query_row("SELECT status FROM approvals WHERE id = 'a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(approval, "denied");
    }

    #[test]
    fn cursor_threads_become_claude_threads() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
        for migration in &MIGRATIONS[..13] {
            apply(&conn, migration).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO threads (id, workspace_id, title, engine, session_id, status, model)
                VALUES ('c', 'w', 'Lead', 'cursor', 'cursor-sess', 'idle', 'gpt-5'),
                       ('k', 'w', 'Lead', 'claude', 'claude-sess', 'idle', 'opus');
             INSERT INTO settings (key, value) VALUES ('defaultEngine', 'cursor');",
        )
        .unwrap();

        run(&conn).unwrap();

        let thread = |id: &str| -> (String, Option<String>, Option<String>) {
            conn.query_row(
                "SELECT engine, session_id, model FROM threads WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
        };
        assert_eq!(thread("c"), ("claude".into(), None, None));
        assert_eq!(
            thread("k"),
            (
                "claude".into(),
                Some("claude-sess".into()),
                Some("opus".into())
            )
        );
        let default: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'defaultEngine'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(default, "claude");
    }
}
