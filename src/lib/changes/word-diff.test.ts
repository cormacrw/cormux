import { describe, expect, it } from 'vitest'
import { wordChangeRanges } from './word-diff'

describe('wordChangeRanges', () => {
  it('finds the middle changed segment', () => {
    const [ra, rb] = wordChangeRanges('hello world', 'hello there')
    expect(ra).toEqual([6, 11])
    expect(rb).toEqual([6, 11])
  })

  it('returns null when most of the line changed', () => {
    const [ra] = wordChangeRanges('abc', 'xyz')
    expect(ra).toBeNull()
  })
})
