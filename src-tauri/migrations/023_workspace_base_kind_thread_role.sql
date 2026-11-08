-- The branch a workspace is compared with. Earlier builds kept it only in memory and fell back
-- to the repo's default branch after a restart, so that's what existing rows get.
ALTER TABLE workspaces ADD COLUMN base TEXT NOT NULL DEFAULT '';
UPDATE workspaces
SET base = COALESCE((SELECT r.default_branch FROM repos r WHERE r.id = workspaces.repo_id), 'main')
WHERE base = '';

-- Every workspace has a kind; a missing one meant a Lead.
UPDATE workspaces SET kind = 'lead' WHERE kind IS NULL OR kind <> 'review';

-- A workspace's own thread was found by its title. The title stays as display text.
ALTER TABLE threads ADD COLUMN role TEXT;
UPDATE threads SET role = 'lead' WHERE title = 'Lead';
UPDATE threads SET role = 'reviewer' WHERE title = 'Reviewer';

-- `waiting` is no longer written; nothing is waiting once the app has restarted.
UPDATE threads SET status = 'idle' WHERE status = 'waiting';
UPDATE workspaces SET status = 'idle' WHERE status = 'waiting';
