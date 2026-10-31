-- What the status queries filter on, so they stop scanning as history grows.
CREATE INDEX IF NOT EXISTS approvals_thread_status ON approvals (thread_id, status);
CREATE INDEX IF NOT EXISTS threads_workspace_id ON threads (workspace_id);
CREATE INDEX IF NOT EXISTS findings_workspace_id ON findings (workspace_id);
CREATE INDEX IF NOT EXISTS thread_events_messages ON thread_events (thread_id, seq) WHERE kind = 'message';
