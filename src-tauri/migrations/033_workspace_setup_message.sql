-- The message a workspace was started with, which a retried setup starts from again. Until now
-- a retry read it back as the first message in the workspace's own thread, so that fills it.
ALTER TABLE workspaces ADD COLUMN setup_message TEXT;

UPDATE workspaces
SET setup_message = (
    SELECT CASE WHEN json_valid(e.payload) THEN json_extract(e.payload, '$.text') END
    FROM thread_events e
    WHERE e.kind = 'message'
      AND e.thread_id = (
          SELECT t.id FROM threads t
          WHERE t.workspace_id = workspaces.id AND t.role IS NOT NULL AND t.closed_at IS NULL
          ORDER BY t.created_at, t.rowid
          LIMIT 1
      )
    ORDER BY e.seq
    LIMIT 1
);
