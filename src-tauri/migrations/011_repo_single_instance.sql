-- When set, running the app in one of the repo's workspaces stops it in the others.
ALTER TABLE repo_config ADD COLUMN single_instance INTEGER NOT NULL DEFAULT 0;
