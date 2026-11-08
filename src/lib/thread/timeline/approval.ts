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
  const payload = row?.payload ?? null
  return {
    kind: 'approval',
    id: event.id,
    title: payload?.title ?? event.title,
    what: payload?.what ?? event.detail ?? event.tool,
    why: payload?.why ?? event.tool,
    okLabel: payload?.okLabel ?? 'Approve',
    noLabel: payload?.noLabel ?? 'Deny',
    state: approvalState(row),
    doneAtMs: payload?.resolvedAtMs ?? null,
    questions: payload?.questions ?? [],
    answers: payload?.answers ?? {},
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
