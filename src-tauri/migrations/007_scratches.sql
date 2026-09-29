-- A scratch owns one thread but no workspace, so threads.workspace_id becomes a plain owner id
-- (a workspace id or a scratch id). Rebuilt because SQLite cannot drop a foreign key in place.
CREATE TABLE threads_new (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    title TEXT NOT NULL,
    engine TEXT NOT NULL,
    session_id TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    used_tokens INTEGER,
    context_size INTEGER,
    cost_usd REAL,
    transcript_readonly INTEGER NOT NULL DEFAULT 0
);

INSERT INTO threads_new (
    id, workspace_id, title, engine, session_id, status, created_at,
    used_tokens, context_size, cost_usd, transcript_readonly
)
SELECT id, workspace_id, title, engine, session_id, status, created_at,
       used_tokens, context_size, cost_usd, transcript_readonly
FROM threads;

DROP TABLE threads;
ALTER TABLE threads_new RENAME TO threads;

CREATE TABLE scratches (
    id TEXT PRIMARY KEY,
    -- No foreign key: removing a repo leaves its scratches in place.
    repo_id TEXT NOT NULL,
    title TEXT NOT NULL,
    thread_id TEXT NOT NULL REFERENCES threads(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
