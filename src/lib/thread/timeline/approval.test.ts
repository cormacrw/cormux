import { describe, expect, it } from 'vitest'
import type { ApprovalRow, TranscriptEvent } from '$lib/ipc/bindings'
import { approvalItem } from './approval'

const request: Extract<TranscriptEvent, { type: 'permission' }> = {
  type: 'permission',
  id: 'p1',
  tool_call_id: null,
  card: {
    title: 'Use Bash',
    what: 'pnpm test',
    why: 'Bash',
    okLabel: 'Approve',
    noLabel: 'Deny',
    questions: [],
    answers: {},
  },
  tool: 'Bash',
  role: { kind: 'command', command: 'pnpm test', description: null },
  detail: 'pnpm test',
  auto_approved: false,
}

describe('approvalItem', () => {
  it('shows the request’s own card until its row arrives', () => {
    const item = approvalItem(request, 1, 0, [])
    expect([item.title, item.what, item.why, item.state]).toEqual([
      'Use Bash',
      'pnpm test',
      'Bash',
      'pending',
    ])
  })

  it('shows the card as the row has it once answered', () => {
    const row: ApprovalRow = {
      id: 'p1',
      threadId: 't1',
      status: 'denied',
      tool: 'execute',
      payload: { ...request.card, resolvedAtMs: 5, denyReason: 'not now' },
    }
    const item = approvalItem(request, 1, 0, [row])
    expect([item.state, item.doneAtMs]).toEqual(['denied', 5])
  })
})
