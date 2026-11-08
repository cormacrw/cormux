import { describe, expect, it } from 'vitest'
import { formatMemoryGb } from './memory'
import { plural } from './plural'

describe('formatMemoryGb', () => {
  it('formats sub-10 GB with one decimal', () => {
    expect(formatMemoryGb(1.4 * 1024 ** 3)).toBe('1.4 GB')
  })
})

describe('plural', () => {
  it('uses singular and plural labels', () => {
    expect(plural(1, 'agent')).toBe('1 agent')
    expect(plural(3, 'agent')).toBe('3 agents')
    expect(plural(2, 'branch', 'branches')).toBe('2 branches')
  })
})
