export type { AgentQuestion } from '$lib/ipc/bindings'

/** What a decided card says instead of its buttons. */
export function approvalResolvedChip(
  item: {
    state: 'approved' | 'denied'
    noLabel: string
    doneAtMs: number | null
    questions: readonly unknown[]
  },
  nowMs: number,
): string {
  const label = formatRelativeApprovalTime(item.doneAtMs ?? nowMs, nowMs)
  if (item.questions.length) {
    return `${item.state === 'approved' ? 'Answered' : 'Skipped'} · ${label}`
  }
  if (item.state === 'approved') return `Approved by you · ${label}`
  return `${item.noLabel} · ${label}`
}

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
