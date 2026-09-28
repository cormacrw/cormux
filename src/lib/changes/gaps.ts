/**
 * COR-151 / hooks for COR-23 (Create PR).
 *
 * Not implemented in COR-22:
 * - Inline diff comments → agent feedback / draft PR review comments
 * - Expand hunk context, whole-file view, per-hunk discard
 * - File filter, directory grouping, thread attribution on edits
 *
 * COR-23 should read `changesReview` marks and warn when opening a PR if any
 * file is still `pending` or `rejected`.
 */
export const CHANGES_REVIEW_GAPS = [
  'inline-line-comments',
  'hunk-context-expand',
  'whole-file-view',
  'per-hunk-discard',
  'file-filter-search',
  'thread-attribution',
] as const
