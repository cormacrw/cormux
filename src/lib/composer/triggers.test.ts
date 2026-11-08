import { describe, expect, it } from 'vitest'
import type { Bang, ClickupTask, RepoRecord } from '$lib/ipc/bindings'
import { pickerAt, replaceWord, wordAt } from './triggers'

describe('wordAt', () => {
  const at = (kind: Parameters<typeof wordAt>[0], draft: string) =>
    wordAt(kind, draft, draft.length)

  it('reads the slash word before the caret, anywhere in the draft', () => {
    expect(at('skill', '/')).toEqual({ start: 0, end: 1, query: '' })
    expect(at('skill', '/dep')?.query).toBe('dep')
    expect(at('skill', 'please run /dep')).toEqual({
      start: 11,
      end: 15,
      query: 'dep',
    })
    expect(at('skill', 'first line\n/rev')?.query).toBe('rev')
  })

  it('covers the rest of the word when the caret is inside it', () => {
    expect(wordAt('skill', '/deploy now', 4)).toEqual({
      start: 0,
      end: 7,
      query: 'dep',
    })
  })

  it('ignores paths and words that are not a slash word', () => {
    expect(at('skill', '')).toBeNull()
    expect(at('skill', 'hi')).toBeNull()
    expect(at('skill', 'src/app')).toBeNull()
    expect(at('skill', '/usr/bin')).toBeNull()
    expect(wordAt('skill', '/usr/bin', 2)).toBeNull()
    expect(at('skill', '/deploy now')).toBeNull()
  })

  it('finds the @ word the caret is in, but not emails or packages', () => {
    expect(at('repo', 'look at @cor')).toEqual({
      start: 8,
      end: 12,
      query: 'cor',
    })
    expect(at('repo', '@')).toEqual({ start: 0, end: 1, query: '' })
    expect(at('repo', 'mail me@example.com')).toBeNull()
    expect(at('repo', 'install @tauri-apps/api')).toBeNull()
    expect(at('repo', 'no at sign')).toBeNull()
  })

  it('finds the # word the caret is in, but not a heading', () => {
    expect(at('task', 'work on #eng')).toEqual({
      start: 8,
      end: 12,
      query: 'eng',
    })
    expect(at('task', '#')).toEqual({ start: 0, end: 1, query: '' })
    expect(at('task', 'issue#12')).toBeNull()
    expect(at('task', '# Heading')).toBeNull()
  })

  it('finds the !word at the caret, at the start or after a space', () => {
    expect(at('bang', '!te')).toEqual({ start: 0, end: 3, query: 'te' })
    expect(wordAt('bang', 'please !re now', 10)).toEqual({
      start: 7,
      end: 10,
      query: 're',
    })
    expect(at('bang', 'wow!te')).toBeNull()
  })
})

describe('replaceWord', () => {
  it('replaces just the word, with a space after it and the caret past that', () => {
    const draft = 'please /rev the diff'
    expect(
      replaceWord(draft, wordAt('skill', draft, 11)!, '/code-review'),
    ).toEqual({
      text: 'please /code-review the diff',
      caret: 20,
    })
    expect(replaceWord('/de', wordAt('skill', '/de', 3)!, '/deploy')).toEqual({
      text: '/deploy ',
      caret: 8,
    })
    expect(
      replaceWord(
        'compare @co with this',
        { start: 8, end: 11, query: 'co' },
        '@cormux',
      ),
    ).toEqual({ text: 'compare @cormux with this', caret: 16 })
    expect(
      replaceWord('do #en', { start: 3, end: 6, query: 'en' }, '#ENG-123'),
    ).toEqual({ text: 'do #ENG-123 ', caret: 12 })
  })

  it('puts text in with no space at the end of the draft', () => {
    const draft = 'please !te'
    expect(
      replaceWord(
        draft,
        wordAt('bang', draft, 10)!,
        'Run the tests and fix what fails.',
        'text',
      ),
    ).toEqual({
      text: 'please Run the tests and fix what fails.',
      caret: 40,
    })
  })
})

describe('pickerAt', () => {
  const repo = {
    id: 'r1',
    name: 'web admin',
    path: '/dev/web-admin',
  } as RepoRecord
  const task = {
    id: 't1',
    customId: 'ENG-1',
    name: 'Retry webhooks',
    url: 'https://app.clickup.com/t/t1',
  } as ClickupTask
  const bang: Bang = { id: 'b1', name: 'test', prompt: ' Run the tests. ' }
  const sources = { skills: null, repos: [repo], tasks: [task], bangs: [bang] }

  it('opens the picker for the word at the caret, with what each option puts in', () => {
    expect(pickerAt('@we', 3, sources)?.options[0]).toMatchObject({
      label: '@web-admin',
      insert: '@web-admin',
      as: 'word',
    })
    expect(pickerAt('#eng', 4, sources)?.options[0]?.label).toBe('#ENG-1')
    expect(pickerAt('!te', 3, sources)?.options[0]).toMatchObject({
      label: '!test',
      insert: 'Run the tests.',
      as: 'text',
    })
    expect(pickerAt('plain words', 11, sources)).toBeNull()
  })

  it('opens skills before they are read, and no tasks or bangs when there are none', () => {
    expect(pickerAt('/', 1, sources)).toMatchObject({
      kind: 'skill',
      options: [],
    })
    const empty = { ...sources, tasks: [], bangs: [] }
    expect(pickerAt('#eng', 4, empty)).toBeNull()
    expect(pickerAt('!te', 3, empty)).toBeNull()
  })
})
