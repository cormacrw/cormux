/**
 * Hooks for COR-26 (Teardown) and follow-up epics.
 */
export const CREATE_PR_GAPS = [
  'warn-on-pending-changes-review-before-create-pr',
  'require-all-files-approved-before-create-pr',
  'auto-teardown-after-merge-when-setting-enabled',
  'pr-status-polling-when-checks-running',
  'branch-switch-clears-stale-pr-badge',
] as const
