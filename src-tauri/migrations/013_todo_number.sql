-- Each TODO gets a short, permanent number, shown as CMUX-<number>, so prompts
-- can refer to it the way they refer to a ClickUp task. Existing TODOs are
-- numbered in the order they were made.
ALTER TABLE todos ADD COLUMN number INTEGER;

UPDATE todos SET number = (
    SELECT COUNT(*) FROM todos AS earlier
    WHERE earlier.created_at < todos.created_at
       OR (earlier.created_at = todos.created_at AND earlier.rowid <= todos.rowid)
);

CREATE UNIQUE INDEX todos_number ON todos (number);
