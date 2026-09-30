/**
 * Hooks for COR-143 (batch approve / per-repo allowlists) and COR-22 changes panel.
 * UI for these is intentionally deferred; the IPC surface exists on the core side.
 */
export type ApprovalAllowlistEntry = {
  pattern: string
  repoId?: string
}

export const APPROVAL_ALLOWLIST_SETTING_KEY = 'approvalCommandAllowlist'

export function parseAllowlist(raw: string | undefined): ApprovalAllowlistEntry[] {
  if (!raw?.trim()) return []
  try {
    const parsed = JSON.parse(raw) as ApprovalAllowlistEntry[]
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}
