export type AgentQuestion = {
  question: string
  header: string
  options: { label: string; description: string }[]
  multiSelect: boolean
}

export type ApprovalPayload = {
  title: string
  what: string
  why: string
  okLabel: string
  noLabel: string
  effect?: string
  resolvedAtMs?: number
  denyReason?: string
  /** What the agent asked; absent on an ordinary permission request. */
  questions?: AgentQuestion[]
  /** The user's answers, keyed by question text. */
  answers?: Record<string, string>
  /** The markdown plan an agent handed over for approval. */
  plan?: string
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
  if (payload.questions?.length) {
    return `${state === 'approved' ? 'Answered' : 'Skipped'} · ${label}`
  }
  if (state === 'approved') return `Approved by you · ${label}`
  return `${payload.noLabel} · ${label}`
}

/** One question's answer: the options picked, then anything typed. */
export function composeAnswer(selected: string[], typed: string): string {
  return [...selected, typed.trim()].filter(Boolean).join(', ')
}

function formatRelativeApprovalTime(atMs: number, nowMs: number): string {
  const delta = Math.max(0, nowMs - atMs)
  if (delta < 45_000) return 'just now'
  const minutes = Math.round(delta / 60_000)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.round(minutes / 60)
  return `${hours}h ago`
}
