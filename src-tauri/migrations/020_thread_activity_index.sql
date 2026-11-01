-- The snapshot's "when did each thread last move" lookup walks a thread's events newest first
-- until it meets a turn end, an exit, or a user message. Mid-turn that meant reading back
-- through every streamed chunk (and parsing its JSON) on every snapshot. This index holds only
-- the rows that count, so the lookup is one seek per thread. Its WHERE clause must match the
-- query in `Store::snapshot` word for word, or SQLite won't use it.
CREATE INDEX IF NOT EXISTS thread_events_activity ON thread_events (thread_id, seq)
    WHERE kind IN ('turn_end', 'exit')
        OR (kind = 'message' AND json_extract(payload, '$.role') = 'user');
