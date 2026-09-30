import { describe, expect, it } from 'vitest'
import {
  readBooleanSetting,
  readDefaultBase,
  readDefaultEngine,
  readStringSetting,
  resolveDefaultRepoId,
} from './settings-defaults'

describe('settings-defaults', () => {
  it('reads engine and base with fallbacks', () => {
    expect(readDefaultEngine([])).toBe('claude')
    expect(readDefaultBase([])).toBe('main')
    expect(
      readDefaultEngine([{ key: 'defaultEngine', value: 'cursor' }]),
    ).toBe('cursor')
    expect(
      readDefaultEngine([{ key: 'defaultEngine', value: 'codex' }]),
    ).toBe('claude')
    expect(readDefaultBase([{ key: 'defaultBase', value: 'develop' }])).toBe(
      'develop',
    )
  })

  it('reads booleans and strings', () => {
    expect(readBooleanSetting([], 'reduceMotion', false)).toBe(false)
    expect(
      readBooleanSetting(
        [{ key: 'reduceMotion', value: 'true' }],
        'reduceMotion',
        false,
      ),
    ).toBe(true)
    expect(readStringSetting([], 'worktreeRoot', '~/.harness/worktrees')).toBe(
      '~/.harness/worktrees',
    )
  })

  it('resolves the default repo, falling back to the first', () => {
    const repos = [{ id: 'a' }, { id: 'b' }]
    expect(resolveDefaultRepoId(repos, 'b')).toBe('b')
    expect(resolveDefaultRepoId(repos, 'gone')).toBe('a')
    expect(resolveDefaultRepoId(repos, '')).toBe('a')
    expect(resolveDefaultRepoId([], 'b')).toBe('')
  })
})
