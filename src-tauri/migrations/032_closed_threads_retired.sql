-- No build has closed a thread since 015 gave each workspace one; those older builds closed are
-- retired too, so whether a thread can be continued has one answer.
UPDATE threads SET retired = 1, status = 'idle' WHERE closed_at IS NOT NULL;
