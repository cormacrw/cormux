/** Documented follow-ups from COR-188 / feature 20. */
export const FINDINGS_KNOWN_GAPS = [
  'Findings cannot be dismissed, edited, re-classified, or commented on',
  'No link from a finding to its diff line (Changes is empty in review workspaces)',
  'Unsent findings do not flow into Submit review as draft comments yet',
  'Fixed is asserted by the agent — no diff view to verify fixes',
  'No way to add your own finding or re-run the review after fixes',
  'Sent findings cannot be recalled',
  'Send to a brand-new thread (e.g. New thread: Fixer) is not in the Send to menu',
] as const
