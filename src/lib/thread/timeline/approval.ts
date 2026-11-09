import type { ApprovalRow, TranscriptEvent } from '$lib/ipc/bindings'
import type { TimelineItem } from '../timeline-types'

type ApprovalItem = Extract<TimelineItem, { kind: 'approval' }>

function approvalState(row: ApprovalRow | undefined): ApprovalItem['state'] {
  if (!row) return 'pending'
  if (row.status === 'approved') return 'approved'
  if (row.status === 'denied') return 'denied'
  return 'pending'
}

// Looked up by id once per list, rather than scanned for every permission in the thread.
const approvalIndexes = new WeakMap<ApprovalRow[], Map<string, ApprovalRow>>()

function approvalById(approvals: ApprovalRow[], id: string) {
  let index = approvalIndexes.get(approvals)
  if (!index) {
    index = new Map(approvals.map((row) => [row.id, row]))
    approvalIndexes.set(approvals, index)
  }
  return index.get(id)
}

export function approvalItem(
  event: Extract<TranscriptEvent, { type: 'permission' }>,
  seq: number,
  atMs: number,
  approvals: ApprovalRow[],
): ApprovalItem {
  const row = approvalById(approvals, event.id)
  // The row is the card as answered; until it arrives (or if it can't be read) the request's own.
  const card = row?.payload ?? event.card
  return {
    kind: 'approval',
    id: event.id,
    title: card.title,
    what: card.what ?? '',
    why: card.why ?? '',
    okLabel: card.okLabel,
    noLabel: card.noLabel,
    state: approvalState(row),
    doneAtMs: card.resolvedAtMs ?? null,
    questions: card.questions ?? [],
    answers: card.answers ?? {},
    seq,
    atMs,
  }
}

/** Whether two approval lists would draw the same cards. */
export function sameApprovals(a: ApprovalRow[], b: ApprovalRow[]) {
  if (a === b) return true
  return (
    a.length === b.length &&
    a.every((row, index) => {
      const other = b[index]
      return (
        row.id === other?.id &&
        row.status === other.status &&
        row.payload === other.payload
      )
    })
  )
}
