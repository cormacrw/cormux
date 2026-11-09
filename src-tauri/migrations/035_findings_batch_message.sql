-- Builds before the batch named its message saved the ids alone. Its message is in the transcript
-- by now; the newest one stands in for it, so every batch reads the same way.
UPDATE threads
SET findings_batch = json_object(
    'ids', json(findings_batch),
    'messageSeq', COALESCE(
        (SELECT MAX(e.seq) FROM thread_events e WHERE e.thread_id = threads.id AND e.kind = 'message'),
        0
    )
)
WHERE json_valid(findings_batch) AND json_type(findings_batch) = 'array';
