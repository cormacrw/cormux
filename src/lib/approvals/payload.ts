export type ApprovalPayload = {
  title: string
  what: string
  why: string
  okLabel: string
  noLabel: string
  effect?: string
  resolvedAtMs?: number
  denyReason?: string
}

export function parseApprovalPayload(raw: string): ApprovalPayload | null {
  if (!raw.trim()) return null
  try {
    const parsed = JSON.parse(raw) as ApprovalPayload
    if (!parsed.title || !parsed.okLabel || !parsed.noLabel) return null
    return parsed
  } catch {
    return null
  }
}

export function approvalResolvedChip(
  payload: ApprovalPayload,
  state: 'approved' | 'denied',
  nowMs: number,
): string {
  const at = payload.resolvedAtMs ?? nowMs
  const label = formatRelativeApprovalTime(at, nowMs)
  if (state === 'approved') return `Approved by you · ${label}`
  return `${payload.noLabel} · ${label}`
}

function formatRelativeApprovalTime(atMs: number, nowMs: number): string {
  const delta = Math.max(0, nowMs - atMs)
  if (delta < 45_000) return 'just now'
  const minutes = Math.round(delta / 60_000)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.round(minutes / 60)
  return `${hours}h ago`
}
