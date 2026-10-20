import { describe, expect, it } from 'vitest'
import { revealEnd, revealRate, stableMarkdown } from './typing'

describe('revealRate', () => {
  it('types a short reply at a readable pace', () => {
    expect(revealRate(40, true)).toBe(90)
  })

  it('catches up on a long reply within about a second, faster once it’s done', () => {
    expect(revealRate(3000, true)).toBe(2500)
    expect(revealRate(3000, false)).toBe(7500)
  })
})

describe('revealEnd', () => {
  const text = 'The webhook handler retries forever.'

  it('finishes the word the reveal stops in', () => {
    expect(text.slice(0, revealEnd(text, 0, 6))).toBe('The webhook')
    expect(text.slice(0, revealEnd(text, 4, 1))).toBe('The webhook')
  })

  it('always moves, and stops at the end', () => {
    expect(revealEnd(text, 0, 0)).toBeGreaterThan(0)
    expect(revealEnd(text, 30, 50)).toBe(text.length)
  })

  it('doesn’t jump far ahead for a very long word', () => {
    const long = `see ${'a'.repeat(40)}`
    expect(revealEnd(long, 0, 6)).toBe(6)
  })
})

describe('stableMarkdown', () => {
  it('closes bold and inline code that are still being typed', () => {
    expect(stableMarkdown('This is **very imp')).toBe('This is **very imp**')
    expect(stableMarkdown('Run `pnpm te')).toBe('Run `pnpm te`')
    expect(stableMarkdown('Done **twice** now')).toBe('Done **twice** now')
    // An asterisk pair inside code isn't bold.
    expect(stableMarkdown('Use `a ** b` and **x')).toBe(
      'Use `a ** b` and **x**',
    )
  })

  it('shows a code block as a block while its lines type in', () => {
    expect(stableMarkdown('Try:\n```ts\nconst a = 1')).toBe(
      'Try:\n```ts\nconst a = 1\n```',
    )
    expect(stableMarkdown('```\nx\n```\nafter')).toBe('```\nx\n```\nafter')
  })

  it('holds back a link until its target is written', () => {
    expect(stableMarkdown('See [the docs](https://exa')).toBe('See ')
    expect(stableMarkdown('See [the do')).toBe('See ')
    expect(stableMarkdown('See [the docs](https://example.com) now')).toBe(
      'See [the docs](https://example.com) now',
    )
  })
})
