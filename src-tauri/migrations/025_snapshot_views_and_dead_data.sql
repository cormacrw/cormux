-- Nothing reads these any more: the TODO and week-plan pages are gone, workspaces'
-- modified-file count was never updated from 0, and no thread was ever made read-only.
DROP TABLE IF EXISTS todos;
DROP TABLE IF EXISTS sprint_plan;
ALTER TABLE workspaces DROP COLUMN modified_files;
ALTER TABLE threads DROP COLUMN transcript_readonly;

-- What the app shows, defined once for every query that lists it. `position` is the table's
-- rowid, which a view doesn't have, for ordering by insertion.
CREATE VIEW live_workspaces AS
    SELECT rowid AS position, * FROM workspaces WHERE archived_at IS NULL;

-- Open threads of live workspaces, and scratches' threads.
CREATE VIEW visible_threads AS
    SELECT rowid AS position, * FROM threads
    WHERE closed_at IS NULL
      AND (workspace_id IN (SELECT id FROM live_workspaces)
           OR id IN (SELECT thread_id FROM scratches));
