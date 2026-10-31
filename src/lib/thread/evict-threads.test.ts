import { describe, expect, it } from 'vitest'
import { threadsToEvict } from './evict-threads'

describe('threadsToEvict', () => {
  it('drops the least recently viewed beyond the limit', () => {
    expect(threadsToEvict(['a', 'b', 'c', 'd'], new Set(), 2)).toEqual([
      'a',
      'b',
    ])
  })

  it('never drops a thread on screen, and counts it toward the limit', () => {
    expect(threadsToEvict(['a', 'b', 'c'], new Set(['a']), 2)).toEqual(['b'])
  })

  it('keeps everything within the limit', () => {
    expect(threadsToEvict(['a', 'b'], new Set(), 2)).toEqual([])
  })
})
