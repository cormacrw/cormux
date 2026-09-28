/** Documented follow-ups from COR-182 / feature 19. */
export const REVIEW_WORKSPACE_KNOWN_GAPS = [
  'No re-review after the author pushes new commits',
  'Review workspaces always use the default engine from Settings',
  'Fixes from findings are local only — pushing to someone else’s PR branch is undecided',
  'Harness MCP tools are not yet wired into live engine sessions (structured review uses the small model fallback)',
] as const
