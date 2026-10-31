-- The thread that started this one through the create_thread tool. No foreign key: a child
-- outlives its parent's workspace being archived or removed.
ALTER TABLE threads ADD COLUMN parent_thread_id TEXT;

CREATE INDEX threads_parent_thread_id ON threads (parent_thread_id);
