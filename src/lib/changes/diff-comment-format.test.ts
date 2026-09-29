import { describe, expect, it } from 'vitest'
import { formatCommentsForAgent, type DiffComment } from './diff-comment-format'

const comment = (over: Partial<DiffComment>): DiffComment => ({
  id: 'x',
  path: 'src/a.ts',
  side: 'new',
  line: 1,
  code: '',
  body: 'note',
  ...over,
})

describe('formatCommentsForAgent', () => {
  it('orders by file then line and quotes the code', () => {
    const text = formatCommentsForAgent([
      comment({ path: 'src/b.ts', line: 3, code: 'const b = 2', body: 'Rename b' }),
      comment({ line: 9, code: '  return x  ', body: ' Handle null ' }),
      comment({ line: 2, side: 'old', code: 'legacy()', body: 'Why remove this?' }),
    ])
    expect(text).toBe(
      [
        'Review comments on your changes:',
        '',
        'src/a.ts:2 (removed line)\n> legacy()\nWhy remove this?',
        '',
        'src/a.ts:9\n>   return x\nHandle null',
        '',
        'src/b.ts:3\n> const b = 2\nRename b',
      ].join('\n'),
    )
  })

  it('omits the quote for blank lines', () => {
    expect(formatCommentsForAgent([comment({ body: 'Add a test' })])).toBe(
      'Review comments on your changes:\n\nsrc/a.ts:1\nAdd a test',
    )
  })
})
