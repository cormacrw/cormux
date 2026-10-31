import { describe, expect, it } from 'vitest'
import { reuseRows } from './reuse-rows'

describe('reuseRows', () => {
  it('returns the previous list when nothing changed', () => {
    const previous = [{ id: 'a', n: 1 }]
    expect(reuseRows(previous, [{ id: 'a', n: 1 }])).toBe(previous)
  })

  it('keeps unchanged rows when another one moved', () => {
    const a = { id: 'a', n: 1 }
    const next = reuseRows(
      [a, { id: 'b', n: 1 }],
      [
        { id: 'a', n: 1 },
        { id: 'b', n: 2 },
      ],
    )
    expect(next[0]).toBe(a)
    expect(next[1]?.n).toBe(2)
  })

  it('notices a reorder', () => {
    const a = { id: 'a' }
    const b = { id: 'b' }
    expect(reuseRows([a, b], [{ ...b }, { ...a }])).toEqual([b, a])
  })
})
