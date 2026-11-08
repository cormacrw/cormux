import { describe, expect, it } from 'vitest'
import {
  parseProse,
  parseReply,
  replySearchText,
  type ReplySegment,
} from './parse'

const parse = (text: string, streaming = false) =>
  parseReply(text, { streaming })
const kinds = (text: string, streaming = false) =>
  parse(text, streaming).map((segment) =>
    segment.kind === 'block' || segment.kind === 'tagBlock'
      ? segment.block.kind
      : segment.kind,
  )
const fence = (lang: string, code: string) => `\`\`\`${lang}\n${code}\n\`\`\``
/** Media segments without their source text, which the assertions don't care about. */
const media = (segments: ReplySegment[]) =>
  segments.map((segment) =>
    segment.kind === 'text'
      ? { kind: 'text', text: segment.raw }
      : segment.kind === 'media'
        ? {
            kind: 'media',
            source: segment.source,
            alt: segment.alt,
            mediaKind: segment.mediaKind,
          }
        : segment,
  )

describe('parseReply blocks', () => {
  it('keeps a plain reply as one piece of prose', () => {
    const text = 'Hello\n\n- a\n- b\n\n| x |\n|---|\n| 1 |\n'
    expect(parse(text)).toEqual([
      { kind: 'text', raw: text, blocks: expect.any(Array) },
    ])
    expect(kinds(fence('ts', 'const a = 1'))).toEqual(['text'])
  })

  it('pulls out the blocks that render richer, keeping the order', () => {
    expect(
      kinds(
        `Before\n\n${fence('diff', '-a\n+b')}\n\nAfter\n\n${fence('mermaid', 'graph TD;A-->B')}`,
      ),
    ).toEqual(['text', 'diff', 'text', 'mermaid'])
  })

  it('leaves a fence inside a list in the list', () => {
    const text = `1. one\n\n   ${fence('diff', '-a').replaceAll('\n', '\n   ')}\n\n2. two`
    expect(kinds(text)).toEqual(['text'])
  })

  it('recognises a run’s output but not any old log', () => {
    expect(kinds(fence('', 'Tests  2 passed (2)'))).toEqual(['run'])
    expect(kinds(fence('', 'hello'))).toEqual(['text'])
  })

  it('only trees JSON long enough to want one', () => {
    const long = JSON.stringify({ a: [1, 2, 3], b: { c: 1 } }, null, 6)
    const longer = `${long.slice(0, -1)}\n\n\n\n\n\n}`
    expect(kinds(fence('json', '{"a":1}'))).toEqual(['text'])
    expect(kinds(fence('json', longer))).toEqual(['json'])
    expect(kinds(fence('json', '{nope\n\n\n\n\n\n\n\n\n\n\n\n'))).toEqual([
      'text',
    ])
  })

  it('takes display math, and leaves a bad chart as code', () => {
    expect(parse('$$E = mc^2$$')).toEqual([
      {
        kind: 'block',
        raw: '$$E = mc^2$$',
        block: { kind: 'math', data: 'E = mc^2' },
      },
    ])
    expect(kinds(fence('chart', '{"type":"pie"}'))).toEqual(['text'])
  })

  it('puts tag blocks after the prose, and never shows the findings', () => {
    expect(
      kinds(
        'Scoped.\n<cormux-tasks>[{"title":"A"}]</cormux-tasks>\nShall I?\n<cormux-findings>[]</cormux-findings>',
      ),
    ).toEqual(['text', 'tasks'])
  })
})

describe('parseReply while streaming', () => {
  it('keeps the block still being written as markdown, and finished ones as blocks', () => {
    const text = `${fence('diff', '-a\n+b')}\n\nThen\n\n${fence('mermaid', 'graph TD;A-->')}`
    expect(kinds(text, true)).toEqual(['diff', 'text'])
    expect(kinds(text)).toEqual(['diff', 'text', 'mermaid'])
  })

  it('splits prose into its top-level blocks', () => {
    const [prose] = parseProse('One\n\n- two\n\nThree', { streaming: true })
    expect(prose).toMatchObject({ kind: 'text' })
    expect(prose?.kind === 'text' && prose.blocks.length).toBe(3)
  })
})

describe('parseReply local media', () => {
  it('pulls a local screenshot out of the prose, in place', () => {
    expect(
      media(
        parse(
          'Here is the result:\n\n![Settings after the fix](/tmp/shots/settings.png)\n\nLooks right.',
        ),
      ),
    ).toEqual([
      { kind: 'text', text: 'Here is the result:\n\n' },
      {
        kind: 'media',
        source: '/tmp/shots/settings.png',
        alt: 'Settings after the fix',
        mediaKind: 'image',
      },
      { kind: 'text', text: '\n\nLooks right.' },
    ])
  })

  it('treats a link to a local recording as a video', () => {
    expect(media(parse('[Watch the flow](demo.webm)'))).toEqual([
      {
        kind: 'media',
        source: 'demo.webm',
        alt: 'Watch the flow',
        mediaKind: 'video',
      },
    ])
  })

  it('accepts file URLs and angle-bracket paths with spaces', () => {
    expect(media(parse('![a](file:///tmp/my%20shots/one.png)'))).toContainEqual(
      {
        kind: 'media',
        source: 'file:///tmp/my%20shots/one.png',
        alt: 'a',
        mediaKind: 'image',
      },
    )
    expect(media(parse('![b](</tmp/my shots/two.mp4>)'))).toContainEqual({
      kind: 'media',
      source: '/tmp/my shots/two.mp4',
      alt: 'b',
      mediaKind: 'video',
    })
  })

  it('accepts raw img and video tags pointing at local files', () => {
    expect(
      media(
        parse(
          '<img src="/tmp/a.png" alt="A"> and <video src="/tmp/b.mov"></video>',
        ),
      ),
    ).toEqual([
      { kind: 'media', source: '/tmp/a.png', alt: 'A', mediaKind: 'image' },
      { kind: 'text', text: ' and ' },
      { kind: 'media', source: '/tmp/b.mov', alt: '', mediaKind: 'video' },
    ])
  })

  it('leaves remote images, document links and code fences alone', () => {
    const text = [
      '![remote](https://example.com/a.png)',
      '[notes](/tmp/notes.pdf)',
      '```md',
      '![example](/tmp/example.png)',
      '```',
    ].join('\n')
    expect(media(parse(text))).toEqual([{ kind: 'text', text }])
  })

  it('leaves a table whole, so it still parses as a table', () => {
    const text = [
      'Compare:',
      '',
      '| Before | After |',
      '|---|---|',
      '| ![a](/tmp/a.png) | ![b](/tmp/b.png) |',
    ].join('\n')
    expect(media(parse(text))).toEqual([{ kind: 'text', text }])
  })

  it('leaves lists whole but still cuts media out of the paragraphs around them', () => {
    const list = '- ![a](/tmp/a.png)\n- second'
    expect(media(parse(`${list}\n\n![b](/tmp/b.png)`))).toEqual([
      { kind: 'text', text: `${list}\n\n` },
      { kind: 'media', source: '/tmp/b.png', alt: 'b', mediaKind: 'image' },
    ])
  })

  it('pairs images captioned before and after, across a blank line', () => {
    const text = '![Before](/a.png)\n\n![After the fix](/b.png)'
    expect(parse(text)).toEqual([
      { kind: 'compare', raw: text, before: '/a.png', after: '/b.png' },
    ])
  })

  it('leaves other images alone', () => {
    expect(kinds('![one](/a.png)\n\n![two](/b.png)')).toEqual([
      'media',
      'text',
      'media',
    ])
  })
})

describe('replySearchText', () => {
  it('reads the prose but not the hidden findings JSON', () => {
    const text = replySearchText(
      parse(
        'One blocking issue.\n<cormux-findings>[{"severity":"blocking","title":"Expiry skipped"}]</cormux-findings>',
      ),
    )
    expect(text).toContain('One blocking issue.')
    expect(text).not.toContain('Expiry')
    expect(text).not.toContain('severity')
  })
})
