import { describe, expect, it } from 'vitest'
import { parseReply, replyProse } from './parse'
import { findTagBlock } from './tag-block'

const shown = (text: string, streaming = false) =>
  replyProse(parseReply(text, { streaming }))

describe('findTagBlock', () => {
  it('takes the last block, as the core does', () => {
    const text = 'a <x>1</x> b <x>2</x> c'
    expect(findTagBlock(text, 'x', false)).toEqual({
      kind: 'block',
      start: 13,
      end: 21,
      body: '2',
    })
  })

  it('strips the code fence agents wrap JSON in', () => {
    expect(
      findTagBlock('<x>\n```json\n[1]\n```\n</x>', 'x', false),
    ).toMatchObject({ body: '[1]' })
  })

  it('holds back an unclosed or half-written tag only while streaming', () => {
    expect(findTagBlock('a <x>[1', 'x', true)).toEqual({
      kind: 'unfinished',
      start: 2,
    })
    expect(findTagBlock('a <', 'x', true)).toEqual({
      kind: 'unfinished',
      start: 2,
    })
    expect(findTagBlock('a <x>[1', 'x', false)).toBeNull()
    expect(findTagBlock('a <', 'x', false)).toBeNull()
  })
})

describe('the findings block', () => {
  it('leaves ordinary replies alone', () => {
    expect(shown('Looks good to me.')).toBe('Looks good to me.')
  })

  it('removes a complete block and the whitespace before it', () => {
    expect(
      shown(
        'Two issues.\n\n<cormux-findings>\n[{"title":"x"}]\n</cormux-findings>',
      ),
    ).toBe('Two issues.')
  })

  it('hides a block that is still streaming', () => {
    expect(shown('Two issues.\n<cormux-findings>\n[{"sev', true)).toBe(
      'Two issues.',
    )
  })

  it('hides a partial opening tag at the end while streaming', () => {
    expect(shown('Two issues.\n<cormux-fi', true)).toBe('Two issues.')
  })

  it('keeps text after the closing tag', () => {
    expect(shown('Done.<cormux-findings>[]</cormux-findings>\nThanks!')).toBe(
      'Done.\nThanks!',
    )
  })

  it('hides only the last block, the one the core reads, leaving a quoted one in the prose', () => {
    const quoted = 'The format is `<cormux-findings>[…]</cormux-findings>`.'
    expect(
      shown(`${quoted}\n\n<cormux-findings>[{"title":"x"}]</cormux-findings>`),
    ).toBe(quoted)
  })

  it('keeps everything after an unclosed tag in a finished reply', () => {
    const reply =
      'Use `<cormux-findings>` to report.\n\nNo issues found, ship it.'
    expect(shown(reply)).toBe(reply)
  })
})
