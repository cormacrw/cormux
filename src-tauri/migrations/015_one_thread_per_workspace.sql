-- A workspace now has one thread. Its most recently active thread carries on; any others stay
-- readable but can't be continued. A scratch's thread is read-only the same way.
ALTER TABLE threads ADD COLUMN retired INTEGER NOT NULL DEFAULT 0;

WITH ranked AS (
    SELECT t.id,
           ROW_NUMBER() OVER (
               PARTITION BY t.workspace_id
               ORDER BY COALESCE((SELECT MAX(e.id) FROM thread_events e WHERE e.thread_id = t.id), 0) DESC,
                        t.rowid DESC
           ) AS position
    FROM threads t
    WHERE t.closed_at IS NULL
)
UPDATE threads
SET retired = 1, status = 'idle'
WHERE id IN (SELECT id FROM ranked WHERE position > 1)
   OR id IN (SELECT thread_id FROM scratches);

-- Nothing can answer these any more, and they would hold the thread in "needs attention".
UPDATE approvals SET status = 'denied'
WHERE status = 'pending' AND thread_id IN (SELECT id FROM threads WHERE retired = 1);
