import { describe, expect, it } from 'vitest'
import { prepareHunk, splitDiffRows } from './parse-diff'

describe('prepareHunk', () => {
  it('assigns line numbers and word ranges', () => {
    const hunk = prepareHunk({
      header: '@@ -1,2 +1,2 @@ fn',
      body: ' context\n-hello world\n+hello there\n',
    })
    expect(hunk.lines[1]?.oldLine).toBe(2)
    expect(hunk.lines[2]?.newLine).toBe(2)
    expect(hunk.lines[1]?.wordRange).toBeTruthy()
  })
})

describe('splitDiffRows', () => {
  it('pairs delete/add blocks for split view', () => {
    const hunk = prepareHunk({
      header: '@@ -1,1 +1,1 @@',
      body: '-a\n+b\n',
    })
    const rows = splitDiffRows(hunk.lines)
    expect(rows).toHaveLength(1)
    expect(rows[0]?.[0]?.text).toBe('a')
    expect(rows[0]?.[1]?.text).toBe('b')
  })
})
