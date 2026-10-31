import { describe, expect, it } from 'vitest'
import { splitReplyBlocks } from './reply-blocks'

const kinds = (text: string) =>
  splitReplyBlocks(text).map((block) => block.kind)
const fence = (lang: string, code: string) => `\`\`\`${lang}\n${code}\n\`\`\``

describe('splitReplyBlocks', () => {
  it('keeps a plain reply as one piece of prose', () => {
    const text = 'Hello\n\n- a\n- b\n\n| x |\n|---|\n| 1 |\n'
    expect(splitReplyBlocks(text)).toEqual([{ kind: 'text', text }])
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
    expect(splitReplyBlocks('$$E = mc^2$$')).toEqual([
      { kind: 'math', tex: 'E = mc^2' },
    ])
    expect(kinds(fence('chart', '{"type":"pie"}'))).toEqual(['text'])
  })
})
