-- The model the user picked for a thread; NULL uses the engine's default.
ALTER TABLE threads ADD COLUMN model TEXT;
