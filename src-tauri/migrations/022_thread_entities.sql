-- Things a thread's agent linked to its work through the attach_entity tool: a PR, a ticket,
-- a doc. One row per URL per thread; attaching the same URL again renames it.
CREATE TABLE thread_entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (thread_id, url)
);
