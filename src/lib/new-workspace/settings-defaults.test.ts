import { describe, expect, it } from 'vitest'
import {
  readBooleanSetting,
  readDefaultBase,
  readDefaultEngine,
  readStringSetting,
} from './settings-defaults'

describe('settings-defaults', () => {
  it('reads engine and base with fallbacks', () => {
    expect(readDefaultEngine([])).toBe('claude')
    expect(readDefaultBase([])).toBe('main')
    expect(
      readDefaultEngine([{ key: 'defaultEngine', value: 'cursor' }]),
    ).toBe('cursor')
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
})
