import { describe, expect, it } from 'vitest'
import { branchPickerLocked, runningAgentCount } from './running-agents'
import type { Thread } from '$lib/state/threads.svelte'

const thread = (patch: Partial<Thread>): Thread => ({
  id: 't1',
  workspaceId: 'ws1',
  role: 'agent',
  engine: 'claude',
  status: 'idle',
  paused: false,
  activity: 'Idle',
  pendingApprovals: 0,
  ...patch,
})

describe('running agents', () => {
  it('counts running and provisioning threads', () => {
    expect(
      runningAgentCount([
        thread({ status: 'running' }),
        thread({ id: 't2', status: 'paused' }),
        thread({ id: 't3', status: 'provisioning' }),
      ]),
    ).toBe(2)
  })

  it('locks the branch picker while agents are active', () => {
    expect(branchPickerLocked([thread({ status: 'running' })])).toBe(true)
    expect(branchPickerLocked([thread({ status: 'idle' })])).toBe(false)
  })
})
