import { describe, expect, it } from 'vitest'
import type { AgentEvent } from '$lib/ipc/bindings'
import { sortThreadList, threadActivityMs, threadUrgency } from './thread-list'

const user: AgentEvent = { type: 'messageChunk', role: 'user', text: 'hi' }
const chunk: AgentEvent = { type: 'messageChunk', role: 'agent', text: 'yo' }
const end: AgentEvent = { type: 'turnEnd', stop_reason: 'end', error: null }

describe('thread activity', () => {
  it('ignores streamed output', () => {
    expect(
      threadActivityMs([
        { atMs: 10, event: user },
        { atMs: 20, event: chunk },
      ]),
    ).toBe(10)
  })

  it('counts a finished turn', () => {
    expect(
      threadActivityMs([
        { atMs: 10, event: user },
        { atMs: 30, event: end },
      ]),
    ).toBe(30)
  })
})

describe('thread urgency', () => {
  const working = {
    lifecycle: 'running' as const,
    paused: false,
    pendingApprovals: 0,
    threadStatus: undefined,
  }

  it('is quiet only while working', () => {
    expect(threadUrgency(working)).toBe('quiet')
    expect(threadUrgency({ ...working, lifecycle: 'provisioning' })).toBe(
      'quiet',
    )
  })

  it('is working as soon as the thread is, before the workspace catches up', () => {
    expect(
      threadUrgency({ ...working, lifecycle: 'idle', threadStatus: 'running' }),
    ).toBe('quiet')
  })

  it('needs attention whenever nothing is happening', () => {
    for (const lifecycle of [
      'idle',
      'ready',
      'waiting',
      'provisioningFailed',
    ] as const) {
      expect(threadUrgency({ ...working, lifecycle })).toBe('attention')
    }
    expect(threadUrgency({ ...working, paused: true })).toBe('attention')
  })

  it('needs attention on an approval, even mid-turn', () => {
    expect(threadUrgency({ ...working, pendingApprovals: 1 })).toBe('attention')
  })
})

describe('thread order', () => {
  it('puts what needs you first, longest wait on top, then recent activity', () => {
    const sorted = sortThreadList([
      { id: 'old', urgency: 'quiet' as const, activityAtMs: 1 },
      { id: 'new', urgency: 'quiet' as const, activityAtMs: 9 },
      { id: 'late', urgency: 'attention' as const, activityAtMs: 8 },
      { id: 'early', urgency: 'attention' as const, activityAtMs: 3 },
    ])
    expect(sorted.map((row) => row.id)).toEqual(['early', 'late', 'new', 'old'])
  })
})
