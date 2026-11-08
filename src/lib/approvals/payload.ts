import { formatRelativeAge } from '$lib/format/relative-time'

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
  const label = formatRelativeAge(item.doneAtMs ?? nowMs, nowMs)
  if (item.questions.length) {
    return `${item.state === 'approved' ? 'Answered' : 'Skipped'} · ${label}`
  }
  if (item.state === 'approved') return `Approved by you · ${label}`
  return `${item.noLabel} · ${label}`
}

export function composeAnswer(selected: string[], typed: string): string {
  return [...selected, typed.trim()].filter(Boolean).join(', ')
}
