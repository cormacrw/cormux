-- The Week page plans TODOs and ClickUp sprint tasks onto days of the week.
ALTER TABLE todos ADD COLUMN description TEXT NOT NULL DEFAULT '';
-- `YYYY-MM-DD`, the local day the task is planned for. NULL sits in the loading area.
ALTER TABLE todos ADD COLUMN planned_on TEXT NULL;
-- SQLite `datetime('now')`, UTC. NULL while open.
ALTER TABLE todos ADD COLUMN done_at TEXT NULL;

-- A local plan for a ClickUp task. Nothing here is written back to ClickUp.
CREATE TABLE sprint_plan (
    task_id TEXT PRIMARY KEY,
    planned_on TEXT NULL,
    done_at TEXT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
