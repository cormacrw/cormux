-- A closed thread tab: hidden from the workspace, its history kept.
ALTER TABLE threads ADD COLUMN closed_at TEXT;
