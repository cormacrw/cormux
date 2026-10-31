import { describe, expect, it } from 'vitest'
import { deepEqual } from './deep-equal'

describe('deepEqual', () => {
  it('compares primitives, arrays and objects by value', () => {
    expect(deepEqual(1, 1)).toBe(true)
    expect(deepEqual('a', 'b')).toBe(false)
    expect(deepEqual(NaN, NaN)).toBe(true)
    expect(deepEqual(null, undefined)).toBe(false)
    expect(deepEqual([1, [2, 3]], [1, [2, 3]])).toBe(true)
    expect(deepEqual([1, 2], [1, 2, 3])).toBe(false)
    expect(deepEqual({ a: { b: 1 } }, { a: { b: 1 } })).toBe(true)
    expect(deepEqual({ a: 1 }, { a: 1, b: undefined })).toBe(false)
    expect(deepEqual({ a: 1 }, { b: 1 })).toBe(false)
    expect(deepEqual([], {})).toBe(false)
  })

  it('agrees with JSON.stringify on timeline-shaped data', () => {
    const item = {
      kind: 'toolRun',
      id: 'run-1',
      steps: [{ kind: 'edit', path: 'a.ts', chips: [{ label: 'x' }] }],
      seq: 3,
    }
    const same = JSON.parse(JSON.stringify(item)) as unknown
    expect(deepEqual(item, same)).toBe(true)
    expect(deepEqual(item, { ...item, seq: 4 })).toBe(false)
  })
})
