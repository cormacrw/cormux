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
})
