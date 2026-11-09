-- Older builds marked an approval card a newer one replaced as dismissed, which read as denied.
UPDATE approvals SET status = 'denied' WHERE status = 'dismissed';
