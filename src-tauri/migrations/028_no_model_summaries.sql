-- No model writes workspace summaries any more: the text left is the intro Cormux writes for a
-- Lead, so the column naming who wrote it goes. A review's author was kept only for the review's
-- summary, which nothing showed.
ALTER TABLE workspaces DROP COLUMN summary_source;
ALTER TABLE review_state DROP COLUMN author;

-- The Lead intro said "no files have been touched yet" for as long as the workspace lived.
UPDATE workspaces
SET summary = 'Started “' || name || '” on a branch from ' || base || '.'
WHERE kind = 'lead' AND summary LIKE 'Just started on “%';

-- A PR review is running from when it's opened until its findings are in, as a branch review
-- is; before, only branch reviews said so.
INSERT INTO review_state (workspace_id, status)
SELECT id, 'running' FROM workspaces WHERE kind = 'review' AND archived_at IS NULL
ON CONFLICT(workspace_id) DO UPDATE SET status = 'running' WHERE review_state.status IS NULL;
