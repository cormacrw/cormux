-- State that belongs to one workspace or thread moves out of the settings table, where it was
-- kept under keys built from the entity's id.

-- A workspace's own run command, set in its Output tab; NULL runs the repo's.
ALTER TABLE workspaces ADD COLUMN run_command TEXT;
UPDATE workspaces SET run_command = (
    SELECT NULLIF(TRIM(value), '') FROM settings
    WHERE key = 'workspace:' || workspaces.id || '/runCommand'
);
DELETE FROM settings WHERE key LIKE 'workspace:%/runCommand';

-- A workspace's review: `status` is `running` while the Reviewer works and `ready` once its
-- findings are in; `author` is the PR author as the cards name them (`you` or `@login`);
-- `submitted` once the review was posted to GitHub.
CREATE TABLE review_state (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
    status TEXT,
    author TEXT,
    submitted INTEGER NOT NULL DEFAULT 0
);
INSERT INTO review_state (workspace_id, status, author, submitted)
SELECT w.id,
       (SELECT value FROM settings WHERE key = 'review:' || w.id || '/status'),
       (SELECT value FROM settings WHERE key = 'review:' || w.id || '/author'),
       COALESCE((SELECT value = 'true' FROM settings WHERE key = 'review:' || w.id || '/submitted'), 0)
FROM workspaces w
WHERE EXISTS (SELECT 1 FROM settings WHERE key LIKE 'review:' || w.id || '/%');
DELETE FROM settings WHERE key LIKE 'review:%';

-- The findings a thread was sent to fix, as a JSON array of ids.
ALTER TABLE threads ADD COLUMN findings_batch TEXT;
UPDATE threads SET findings_batch = (
    SELECT value FROM settings WHERE key = 'findings-batch:' || threads.id
);
DELETE FROM settings WHERE key LIKE 'findings-batch:%';
