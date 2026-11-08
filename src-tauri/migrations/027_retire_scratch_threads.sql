-- Scratches can't be continued any more, but those made after 015 weren't marked so: their
-- threads become read-only like every other scratch's.
UPDATE threads
SET retired = 1, status = 'idle'
WHERE id IN (SELECT thread_id FROM scratches);

UPDATE approvals SET status = 'denied'
WHERE status = 'pending' AND thread_id IN (SELECT thread_id FROM scratches);
