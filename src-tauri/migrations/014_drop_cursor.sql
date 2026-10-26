-- Cursor is no longer an engine. Its threads carry on as Claude threads: the history stays,
-- but a Cursor session and model mean nothing to Claude, so the next turn starts afresh.
UPDATE threads SET engine = 'claude', session_id = NULL, model = NULL
WHERE lower(engine) = 'cursor';

UPDATE settings SET value = 'claude'
WHERE key = 'defaultEngine' AND lower(value) = 'cursor';
