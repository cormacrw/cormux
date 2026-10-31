import { describe, expect, it } from 'vitest'
import { findFileRefs, wholeFileRef, worktreePath } from './file-refs'

const paths = (text: string) => findFileRefs(text).map((ref) => ref.path)

describe('findFileRefs', () => {
  it('finds paths, with or without a line', () => {
    expect(findFileRefs('See src/lib/typing.ts:40 for it')).toEqual([
      { index: 4, length: 20, path: 'src/lib/typing.ts', line: 40 },
    ])
    expect(paths('in ./a/b.svelte and /tmp/x/y.png')).toEqual([
      './a/b.svelte',
      '/tmp/x/y.png',
    ])
    expect(paths('at typing.ts:40')).toEqual(['typing.ts'])
  })

  it('leaves words and URLs alone', () => {
    expect(paths('built on node.js, e.g. this one')).toEqual([])
    expect(paths('and/or either/or')).toEqual([])
    expect(paths('see https://example.com/a/b.html now')).toEqual([])
  })
})

describe('wholeFileRef', () => {
  it('only matches a ref that is the whole text', () => {
    expect(wholeFileRef('src/a.ts:3')?.line).toBe(3)
    expect(wholeFileRef('cat src/a.ts')).toBeNull()
  })
})

describe('worktreePath', () => {
  it('makes a path relative to the worktree', () => {
    expect(worktreePath('/w/t/src/a.ts', '/w/t')).toBe('src/a.ts')
    expect(worktreePath('./src/a.ts', '/w/t')).toBe('src/a.ts')
    expect(worktreePath('src/a.ts', undefined)).toBe('src/a.ts')
  })
})
