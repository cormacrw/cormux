/**
 * COR-160 — deliberate gaps vs product/features/15-run-app-and-output.md.
 */
export const RUN_APP_GAPS = [
  'multiple-named-run-targets',
  'per-workspace-run-command-override',
  'log-search-copy-save',
  'sub-path-and-https-urls',
  'dev-server-keypress-forwarding',
  'in-app-browser-preview',
  'apps-survive-harness-quit',
] as const

/**
 * COR-24 (Create PR) should read `changesReview` and warn when any file is still
 * pending or rejected before opening a PR (see also CHANGES_REVIEW_GAPS in changes/gaps.ts).
 */
export const CREATE_PR_FROM_RUN_APP_HOOK =
  'warn-on-pending-changes-review-before-create-pr'
