-- Codex and Gemini are no longer engines. Their threads carry on as Claude threads: the history
-- stays, but their sessions and models mean nothing to Claude, so the next turn starts afresh.
UPDATE threads SET engine = 'claude', session_id = NULL, model = NULL
WHERE lower(engine) IN ('codex', 'gemini');

UPDATE settings SET value = 'claude'
WHERE key = 'defaultEngine' AND lower(value) IN ('codex', 'gemini');
