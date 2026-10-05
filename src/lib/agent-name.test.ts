import { describe, expect, it } from 'vitest'
import { agentName } from './agent-name'

describe('agentName', () => {
  it('stays the same for a thread and can differ between threads', () => {
    expect(agentName('th-lead')).toBe(agentName('th-lead'))
    const names = ['th-lead', 'th-a', 'th-b', 'ws-1', 'scratch-1'].map(
      agentName,
    )
    expect(new Set(names).size).toBeGreaterThan(1)
  })
})
