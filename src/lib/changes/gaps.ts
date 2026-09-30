/**
 * COR-151. Not implemented yet:
 * - Comments on a range of lines (git-diff-view's DiffViewWithMultiSelect), or
 *   drafting them as PR review comments
 * - Expand hunk context / whole-file view (needs old and new file contents)
 * - Per-hunk discard, file filter, directory grouping, thread attribution
 */
export const CHANGES_REVIEW_GAPS = [
  'multi-line-comments',
  'hunk-context-expand',
  'whole-file-view',
  'per-hunk-discard',
  'file-filter-search',
  'thread-attribution',
] as const
