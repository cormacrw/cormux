import { describe, expect, it } from 'vitest'
import type { Snapshot } from '$lib/ipc/bindings'
import { createSupervisor } from './supervisor'
import type { ToastPayload } from './toast-payload'

function snapshot({
  pending = 0,
  openFindings = 0,
}: { pending?: number; openFindings?: number } = {}): Snapshot {
  return {
    pendingApprovals: pending,
    settings: { notifyApprovals: true, notifyReviewFinished: true },
    persisted: {
      findings: Array.from({ length: openFindings }, (_, index) => ({
        id: `f${index}`,
        workspaceId: 'ws-review',
        status: 'open',
      })),
      workspaces: [{ id: 'ws-review', prNumber: 12 }],
      approvals: [],
      threads: [],
      scratches: [],
    },
  } as unknown as Snapshot
}

function supervisor(focused = false) {
  const toasts: ToastPayload[] = []
  const notified: string[] = []
  const supervise = createSupervisor({
    toast: (payload) => void toasts.push(payload),
    notify: async (body) => void notified.push(body),
    focused: () => focused,
  })
  return { supervise, toasts, notified }
}

describe('createSupervisor', () => {
  it('takes what was already there at launch as the baseline', async () => {
    const { supervise, toasts, notified } = supervisor()
    await supervise(snapshot({ pending: 2, openFindings: 3 }))
    expect(toasts).toEqual([])
    expect(notified).toEqual([])
  })

  it('says a review finished once, when its findings first come in', async () => {
    const { supervise, toasts, notified } = supervisor()
    await supervise(snapshot())
    await supervise(snapshot({ openFindings: 3 }))
    await supervise(snapshot({ openFindings: 4 }))
    expect(toasts).toHaveLength(1)
    expect(toasts[0]!.parts.map((part) => part.value).join('')).toBe(
      'Review of #12 finished · 3 findings',
    )
    expect(notified).toEqual(['Review of #12 finished · 3 findings'])
  })

  it('notifies about a new approval only while the window is in the background', async () => {
    const away = supervisor(false)
    await away.supervise(snapshot())
    await away.supervise(snapshot({ pending: 1 }))
    expect(away.notified).toEqual(['An agent is waiting for your approval.'])

    const here = supervisor(true)
    await here.supervise(snapshot())
    await here.supervise(snapshot({ pending: 1 }))
    expect(here.notified).toEqual([])
  })
})
