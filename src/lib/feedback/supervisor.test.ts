import { describe, expect, it } from 'vitest'
import type { Snapshot } from '$lib/ipc/bindings'
import { createSupervisor } from './supervisor'

function snapshot({ pending = 0 }: { pending?: number } = {}): Snapshot {
  return {
    pendingApprovals: pending,
    settings: { notifyApprovals: true },
    persisted: {
      findings: [{ id: 'f1', workspaceId: 'ws-review', status: 'open' }],
      workspaces: [{ id: 'ws-review', prNumber: 12 }],
      approvals: [],
      threads: [],
      scratches: [],
    },
  } as unknown as Snapshot
}

function supervisor(focused = false) {
  const notified: string[] = []
  const supervise = createSupervisor({
    notify: async (body) => void notified.push(body),
    focused: () => focused,
  })
  return { supervise, notified }
}

describe('createSupervisor', () => {
  it('takes what was already there at launch as the baseline', async () => {
    const { supervise, notified } = supervisor()
    await supervise(snapshot({ pending: 2 }))
    expect(notified).toEqual([])
  })

  // Bug: findings coming back from dismissed read as a review that just finished.
  it('says nothing about reviews: open findings changing is not a review finishing', async () => {
    const { supervise, notified } = supervisor()
    const none = snapshot()
    await supervise({ ...none, persisted: { ...none.persisted, findings: [] } })
    await supervise(snapshot())
    expect(notified).toEqual([])
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
