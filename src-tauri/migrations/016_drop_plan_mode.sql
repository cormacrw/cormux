-- Plan mode is gone, so nothing can answer a plan still waiting for review.
UPDATE approvals SET status = 'denied' WHERE status = 'pending' AND tool = 'execute_plan';
UPDATE threads SET status = 'idle' WHERE status = 'waiting';
UPDATE workspaces SET status = 'idle' WHERE status = 'waiting';
