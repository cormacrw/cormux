use rusqlite::Connection;

use crate::error::{Error, Result};

struct Migration {
    /// The file name without `.sql`; its number is the version.
    name: &'static str,
    sql: &'static str,
}

/// A migration that rewrites rows in Rust rather than running a file. The module whose rows it
/// reads supplies it, and it runs in version order with the files.
pub struct RustMigration {
    /// Named like a file, so its first three digits are its version.
    pub name: &'static str,
    pub run: fn(&Connection) -> Result<()>,
}

fn version(name: &str) -> i64 {
    name[..3]
        .parse()
        .expect("a migration's name starts with its three-digit version")
}

enum Step {
    Sql(&'static str),
    Rust(fn(&Connection) -> Result<()>),
}

/// Every migration, files and `rust` together, in version order. A version taken twice is a
/// build mistake.
fn ordered(rust: &[RustMigration]) -> Vec<(i64, &'static str, Step)> {
    let mut all: Vec<(i64, &'static str, Step)> = MIGRATIONS
        .iter()
        .map(|m| (version(m.name), m.name, Step::Sql(m.sql)))
        .chain(
            rust.iter()
                .map(|m| (version(m.name), m.name, Step::Rust(m.run))),
        )
        .collect();
    all.sort_by_key(|(version, ..)| *version);
    assert!(
        all.windows(2).all(|pair| pair[0].0 != pair[1].0),
        "two migrations share a version"
    );
    all
}

/// Every version the migrations take, in order.
#[cfg(test)]
pub fn versions(rust: &[RustMigration]) -> Vec<i64> {
    ordered(rust)
        .into_iter()
        .map(|(version, ..)| version)
        .collect()
}

macro_rules! migrations {
    ($($name:literal),* $(,)?) => {
        &[$(Migration {
            name: $name,
            sql: include_str!(concat!("../../migrations/", $name, ".sql")),
        }),*]
    };
}

const MIGRATIONS: &[Migration] = migrations![
    "001_initial",
    "002_sessions_findings_usage",
    "003_workspace_card_fields",
    "004_workspace_archive",
    "005_workspace_pr_url",
    "006_findings_sent_thread",
    "007_scratches",
    "008_thread_closed",
    "009_todos",
    "010_thread_model",
    "011_repo_single_instance",
    "012_week_plan",
    "013_todo_number",
    "014_drop_cursor",
    "015_one_thread_per_workspace",
    "016_drop_plan_mode",
    "017_thread_parent",
    "018_drop_acp_engines",
    "019_read_path_indexes",
    "020_thread_activity_index",
    "021_thread_effort",
    "022_thread_entities",
    "023_workspace_base_kind_thread_role",
    "025_snapshot_views_and_dead_data",
    "026_per_entity_state",
    "027_retire_scratch_threads",
    "028_no_model_summaries",
    "029_drop_pr_cache_synced_at",
];

fn create_ledger(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;
    Ok(())
}

pub fn run(conn: &Connection, rust: &[RustMigration]) -> Result<()> {
    create_ledger(conn)?;

    // Table rebuilds (007) drop a parent table, which would cascade-delete its children while
    // foreign keys are on. The pragma is ignored inside a transaction, so set it out here.
    conn.pragma_update(None, "foreign_keys", false)?;
    let result = apply_pending(conn, rust);
    conn.pragma_update(None, "foreign_keys", true)?;
    result
}

fn apply_pending(conn: &Connection, rust: &[RustMigration]) -> Result<()> {
    for (version, name, step) in ordered(rust) {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get(0),
        )?;

        if already_applied {
            continue;
        }

        conn.execute_batch("BEGIN IMMEDIATE;")?;
        if let Err(error) = apply(conn, version, name, &step) {
            conn.execute_batch("ROLLBACK;").ok();
            return Err(error);
        }
        conn.execute_batch("COMMIT;")?;
        log::info!("applied migration {name}");
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

fn apply(conn: &Connection, version: i64, name: &str, step: &Step) -> Result<()> {
    let applied = match step {
        Step::Sql(sql) => conn.execute_batch(sql).map_err(Error::from),
        Step::Rust(run) => run(conn),
    };
    applied.map_err(|error| Error::Store(format!("{name}: {error}")))?;
    conn.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        rusqlite::params![version, name],
    )?;
    Ok(())
}

/// A database as the migrations before `version` left it, for testing the ones after.
#[cfg(test)]
pub fn migrated_before(conn: &Connection, version: i64) {
    create_ledger(conn).unwrap();
    for migration in MIGRATIONS
        .iter()
        .filter(|m| self::version(m.name) < version)
    {
        apply(
            conn,
            self::version(migration.name),
            migration.name,
            &Step::Sql(migration.sql),
        )
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_before(version: i64) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        super::migrated_before(&conn, version);
        conn
    }

    /// Runs the one migration numbered `version`, on a database migrated up to it.
    fn apply_version(conn: &Connection, version: i64) {
        let migration = MIGRATIONS
            .iter()
            .find(|m| super::version(m.name) == version)
            .unwrap();
        apply(conn, version, migration.name, &Step::Sql(migration.sql)).unwrap();
    }

    fn names(conn: &Connection, kind: &str) -> Vec<String> {
        conn.prepare(
            "SELECT name FROM sqlite_master
             WHERE type = ?1 AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .unwrap()
        .query_map([kind], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect()
    }

    fn columns(conn: &Connection, table: &str) -> Vec<String> {
        conn.prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|name| name.unwrap())
            .collect()
    }

    #[test]
    fn every_migration_file_is_listed_in_order() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter_map(|name| name.strip_suffix(".sql").map(str::to_string))
            .collect();
        files.sort();
        let listed: Vec<&str> = MIGRATIONS.iter().map(|m| m.name).collect();
        assert_eq!(listed, files);
    }

    #[test]
    fn a_rust_migration_runs_in_version_order_with_the_files() {
        fn saw_views(conn: &Connection) -> Result<()> {
            let views: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'view'",
                [],
                |row| row.get(0),
            )?;
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('views', ?1)",
                [views],
            )?;
            Ok(())
        }
        let rust = [RustMigration {
            name: "024_check",
            run: saw_views,
        }];
        let conn = migrated_before(24);
        run(&conn, &rust).unwrap();
        run(&conn, &rust).unwrap();

        let views: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'views'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(views, "0", "ran before 025 made the views");
        let ledger: Vec<i64> = conn
            .prepare("SELECT version FROM schema_migrations WHERE version >= 23 ORDER BY rowid")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|version| version.unwrap())
            .collect();
        assert_eq!(ledger[..3], [23, 24, 25]);
    }

    #[test]
    fn applies_every_migration_once() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &[]).unwrap();
        run(&conn, &[]).unwrap();

        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(applied as usize, MIGRATIONS.len());
        assert_eq!(
            names(&conn, "table"),
            [
                "approvals",
                "findings",
                "pr_cache",
                "repo_config",
                "repos",
                "review_state",
                "schema_migrations",
                "scratches",
                "settings",
                "thread_entities",
                "thread_events",
                "threads",
                "workspaces",
            ]
        );
        assert_eq!(names(&conn, "view"), ["live_workspaces", "visible_threads"]);
    }

    #[test]
    fn scratches_migration_keeps_thread_history() {
        let conn = migrated_before(7);
        conn.pragma_update(None, "foreign_keys", true).unwrap();
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

        run(&conn, &[]).unwrap();

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
        let conn = migrated_before(15);
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

        run(&conn, &[]).unwrap();

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
        let conn = migrated_before(14);
        conn.execute_batch(
            "INSERT INTO threads (id, workspace_id, title, engine, session_id, status, model)
                VALUES ('c', 'w', 'Lead', 'cursor', 'cursor-sess', 'idle', 'gpt-5'),
                       ('k', 'w', 'Lead', 'claude', 'claude-sess', 'idle', 'opus');
             INSERT INTO settings (key, value) VALUES ('defaultEngine', 'cursor');",
        )
        .unwrap();

        run(&conn, &[]).unwrap();

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

    #[test]
    fn codex_and_gemini_threads_become_claude_threads() {
        let conn = migrated_before(18);
        conn.execute_batch(
            "INSERT INTO threads (id, workspace_id, title, engine, session_id, status, model)
                VALUES ('x', 'w', 'Lead', 'codex', 'acp-sess', 'idle', 'gpt-5'),
                       ('g', 'w', 'Lead', 'Gemini', 'acp-sess', 'idle', 'flash'),
                       ('k', 'w', 'Lead', 'claude', 'claude-sess', 'idle', 'opus');
             INSERT INTO settings (key, value) VALUES ('defaultEngine', 'gemini');",
        )
        .unwrap();

        run(&conn, &[]).unwrap();

        let thread = |id: &str| -> (String, Option<String>, Option<String>) {
            conn.query_row(
                "SELECT engine, session_id, model FROM threads WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
        };
        assert_eq!(thread("x"), ("claude".into(), None, None));
        assert_eq!(thread("g"), ("claude".into(), None, None));
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

    #[test]
    fn workspaces_get_a_base_and_kind_and_threads_a_role() {
        let conn = migrated_before(23);
        conn.execute_batch(
            "INSERT INTO repos (id, path, name, default_branch) VALUES
                ('dev', '/tmp/dev', 'dev', 'develop'),
                ('plain', '/tmp/plain', 'plain', NULL);
             INSERT INTO workspaces (id, repo_id, name, branch, worktree_path, status, kind) VALUES
                ('lead', 'dev', 'Lead work', 'feat/x', '/tmp/x', 'waiting', NULL),
                ('review', 'dev', 'PR #2 Review', 'pr-2', '/tmp/pr', 'idle', 'review'),
                ('other', 'plain', 'Other', 'feat/y', '/tmp/y', 'ready', NULL);
             INSERT INTO threads (id, workspace_id, title, engine, status) VALUES
                ('l', 'lead', 'Lead', 'claude', 'waiting'),
                ('r', 'review', 'Reviewer', 'claude', 'idle'),
                ('h', 'other', 'Agent 2', 'claude', 'idle');",
        )
        .unwrap();

        run(&conn, &[]).unwrap();

        let workspace = |id: &str| -> (String, String, String) {
            conn.query_row(
                "SELECT base, kind, status FROM workspaces WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
        };
        assert_eq!(
            workspace("lead"),
            ("develop".into(), "lead".into(), "idle".into())
        );
        assert_eq!(
            workspace("review"),
            ("develop".into(), "review".into(), "idle".into())
        );
        assert_eq!(
            workspace("other"),
            ("main".into(), "lead".into(), "ready".into())
        );
        let thread = |id: &str| -> (Option<String>, String, String) {
            conn.query_row(
                "SELECT role, title, status FROM threads WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
        };
        assert_eq!(
            thread("l"),
            (Some("lead".into()), "Lead".into(), "idle".into())
        );
        assert_eq!(thread("r").0.as_deref(), Some("reviewer"));
        assert_eq!(thread("h"), (None, "Agent 2".into(), "idle".into()));
    }

    #[test]
    fn dead_tables_and_columns_go_and_the_views_list_what_shows() {
        let conn = migrated_before(25);
        conn.execute_batch(
            "INSERT INTO todos (id, title) VALUES ('t', 'old');
             INSERT INTO repos (id, path, name) VALUES ('r', '/tmp/r', 'r');
             INSERT INTO workspaces (id, repo_id, name, branch, worktree_path, status, modified_files)
                VALUES ('live', 'r', 'Live', 'a', '/tmp/a', 'idle', 3),
                       ('gone', 'r', 'Gone', 'b', '/tmp/b', 'idle', 0);
             UPDATE workspaces SET archived_at = datetime('now') WHERE id = 'gone';
             INSERT INTO threads (id, workspace_id, title, engine, status, transcript_readonly) VALUES
                ('open', 'live', 'Lead', 'claude', 'idle', 1),
                ('closed', 'live', 'Agent 2', 'claude', 'idle', 0),
                ('archived', 'gone', 'Lead', 'claude', 'idle', 0),
                ('scratch', 's', 'Scratch', 'claude', 'idle', 0);
             UPDATE threads SET closed_at = datetime('now') WHERE id = 'closed';
             INSERT INTO scratches (id, repo_id, title, thread_id) VALUES ('s', 'r', 'Q', 'scratch');",
        )
        .unwrap();

        run(&conn, &[]).unwrap();

        assert!(!names(&conn, "table").contains(&"todos".to_string()));
        assert!(!columns(&conn, "workspaces").contains(&"modified_files".to_string()));
        assert!(!columns(&conn, "threads").contains(&"transcript_readonly".to_string()));
        let ids = |sql: &str| -> Vec<String> {
            conn.prepare(sql)
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .map(|id| id.unwrap())
                .collect()
        };
        assert_eq!(ids("SELECT id FROM live_workspaces"), ["live"]);
        assert_eq!(
            ids("SELECT id FROM visible_threads ORDER BY position"),
            ["open", "scratch"]
        );
    }

    #[test]
    fn per_entity_settings_move_to_their_rows() {
        let conn = migrated_before(26);
        conn.execute_batch(
            "INSERT INTO repos (id, path, name) VALUES ('r', '/tmp/r', 'r');
             INSERT INTO workspaces (id, repo_id, name, branch, worktree_path, status) VALUES
                ('own', 'r', 'Own', 'a', '/tmp/a', 'idle'),
                ('plain', 'r', 'Plain', 'b', '/tmp/b', 'idle');
             INSERT INTO threads (id, workspace_id, title, engine, status) VALUES
                ('t', 'own', 'Lead', 'claude', 'idle');
             INSERT INTO settings (key, value) VALUES
                ('workspace:own/runCommand', ' pnpm dev --port 4000 '),
                ('workspace:plain/runCommand', '  '),
                ('review:own/status', 'ready'),
                ('review:own/author', '@ana'),
                ('review:own/submitted', 'true'),
                ('review:gone/status', 'running'),
                ('findings-batch:t', '[\"f1\",\"f2\"]'),
                ('reduceMotion', 'true');",
        )
        .unwrap();

        apply_version(&conn, 26);

        let run_command = |id: &str| -> Option<String> {
            conn.query_row(
                "SELECT run_command FROM workspaces WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(run_command("own").as_deref(), Some("pnpm dev --port 4000"));
        assert_eq!(run_command("plain"), None);
        let review: (String, String, bool) = conn
            .query_row(
                "SELECT status, author, submitted FROM review_state WHERE workspace_id = 'own'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(review, ("ready".into(), "@ana".into(), true));
        let reviews: i64 = conn
            .query_row("SELECT COUNT(*) FROM review_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(reviews, 1, "a review of a workspace that's gone isn't kept");
        let batch: String = conn
            .query_row(
                "SELECT findings_batch FROM threads WHERE id = 't'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(batch, r#"["f1","f2"]"#);
        let keys: Vec<String> = conn
            .prepare("SELECT key FROM settings")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|key| key.unwrap())
            .collect();
        assert_eq!(keys, ["reduceMotion"]);
    }

    #[test]
    fn every_scratch_thread_is_read_only() {
        let conn = migrated_before(27);
        conn.execute_batch(
            "INSERT INTO threads (id, workspace_id, title, engine, status) VALUES
                ('late', 's', 'Scratch', 'claude', 'running'),
                ('lead', 'w', 'Lead', 'claude', 'running');
             INSERT INTO scratches (id, repo_id, title, thread_id)
                VALUES ('s', 'r', 'A question', 'late');
             INSERT INTO approvals (id, thread_id, status, tool, payload) VALUES
                ('a', 'late', 'pending', 'bash', '{}'),
                ('b', 'lead', 'pending', 'bash', '{}');",
        )
        .unwrap();

        run(&conn, &[]).unwrap();

        let thread = |id: &str| -> (bool, String) {
            conn.query_row(
                "SELECT retired, status FROM threads WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
        };
        assert_eq!(thread("late"), (true, "idle".into()));
        assert_eq!(thread("lead"), (false, "running".into()));
        let approval = |id: &str| -> String {
            conn.query_row("SELECT status FROM approvals WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap()
        };
        assert_eq!(
            (approval("a"), approval("b")),
            ("denied".into(), "pending".into())
        );
    }

    #[test]
    fn no_column_says_a_model_wrote_a_summary_and_old_lead_intros_stop_claiming_nothing_changed() {
        let conn = migrated_before(28);
        conn.execute_batch(
            "INSERT INTO repos (id, path, name) VALUES ('r', '/tmp/r', 'r');
             INSERT INTO workspaces (id, repo_id, name, branch, base, worktree_path, status, kind, summary)
             VALUES
                ('lead', 'r', 'Login', 'a', 'main', '/tmp/a', 'idle', 'lead',
                 'Just started on “Login”, branched from main. Claude Code is reading the repo to find where the change belongs; no files have been touched yet.'),
                ('kept', 'r', 'Kept', 'b', 'main', '/tmp/b', 'idle', 'lead', 'Adds OAuth.'),
                ('pr', 'r', 'PR #7 Review', 'c', 'main', '/tmp/c', 'running', 'review', NULL),
                ('done', 'r', 'PR #8 Review', 'd', 'main', '/tmp/d', 'idle', 'review', NULL);
             INSERT INTO review_state (workspace_id, status, author) VALUES
                ('lead', 'ready', '@ana'),
                ('done', 'ready', 'you');",
        )
        .unwrap();

        run(&conn, &[]).unwrap();

        assert!(!columns(&conn, "workspaces").contains(&"summary_source".to_string()));
        assert!(!columns(&conn, "review_state").contains(&"author".to_string()));
        let summary = |id: &str| -> String {
            conn.query_row(
                "SELECT summary FROM workspaces WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(summary("lead"), "Started “Login” on a branch from main.");
        assert_eq!(summary("kept"), "Adds OAuth.");
        let ids: Vec<String> = conn
            .prepare("SELECT id FROM live_workspaces ORDER BY position")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|id| id.unwrap())
            .collect();
        assert_eq!(ids, ["lead", "kept", "pr", "done"], "the view still reads");
        let status = |id: &str| -> String {
            conn.query_row(
                "SELECT status FROM review_state WHERE workspace_id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(status("pr"), "running", "an open PR review is running");
        assert_eq!(status("done"), "ready");
    }

    #[test]
    fn a_cached_pull_request_keeps_its_row_without_a_sync_time() {
        let conn = migrated_before(29);
        conn.execute_batch(
            "INSERT INTO repos (id, path, name) VALUES ('r', '/tmp/r', 'r');
             INSERT INTO pr_cache (id, repo_id, number, title, payload)
             VALUES ('o/r#1', 'r', 1, 'One', '{}');",
        )
        .unwrap();

        run(&conn, &[]).unwrap();

        assert!(!columns(&conn, "pr_cache").contains(&"synced_at".to_string()));
        let title: String = conn
            .query_row("SELECT title FROM pr_cache WHERE id = 'o/r#1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "One");
    }
}
