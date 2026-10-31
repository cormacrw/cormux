import { describe, expect, it } from 'vitest'
import { parseDiffBlock } from './diff-block'

describe('parseDiffBlock', () => {
  it('counts changes and finds the file', () => {
    const diff = parseDiffBlock(
      'diff --git a/src/a.ts b/src/a.ts\n--- a/src/a.ts\n+++ b/src/a.ts\n@@ -1 +1 @@\n-old\n+new\n same\n',
    )
    expect(diff).toMatchObject({ path: 'src/a.ts', added: 1, deleted: 1 })
    expect(diff.lines.map((line) => line.kind)).toEqual([
      'hunk',
      'del',
      'add',
      'context',
    ])
  })

  it('copes with a bare snippet', () => {
    expect(parseDiffBlock('-a\n+b').path).toBeNull()
  })
})
