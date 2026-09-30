ALTER TABLE workspaces ADD COLUMN summary TEXT;
ALTER TABLE workspaces ADD COLUMN summary_at TEXT;
ALTER TABLE workspaces ADD COLUMN summary_source TEXT NOT NULL DEFAULT 'Haiku 4.5';
ALTER TABLE workspaces ADD COLUMN kind TEXT;
ALTER TABLE workspaces ADD COLUMN pr_number INTEGER;
ALTER TABLE workspaces ADD COLUMN modified_files INTEGER NOT NULL DEFAULT 0;
