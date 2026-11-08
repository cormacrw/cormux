-- Review records whether its last finished review found anything, instead of reading findings.
ALTER TABLE review_state ADD COLUMN has_findings INTEGER NOT NULL DEFAULT 0;
UPDATE review_state SET has_findings = 1
WHERE EXISTS (SELECT 1 FROM findings WHERE findings.workspace_id = review_state.workspace_id);
