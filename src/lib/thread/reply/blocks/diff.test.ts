import { describe, expect, it } from 'vitest'
import { parseReply } from '../parse'

/** The diff a reply's fenced `diff` block shows. */
function diffIn(code: string) {
  const [segment] = parseReply(`\`\`\`diff\n${code}\n\`\`\``, {
    streaming: false,
  })
  if (segment?.kind !== 'block' || segment.block.kind !== 'diff') {
    throw new Error('not a diff block')
  }
  return segment.block.data
}

describe('a diff block', () => {
  it('counts changes and names the file, without repeating its header', () => {
    const diff = diffIn(
      'diff --git a/src/a.ts b/src/a.ts\n--- a/src/a.ts\n+++ b/src/a.ts\n@@ -1 +1 @@\n-old\n+new\n same',
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
    expect(diffIn('-a\n+b').path).toBeNull()
  })
})
