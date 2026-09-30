import { describe, expect, it } from 'vitest'
import {
  isRunnableMacro,
  macroNamed,
  macroPromptPreview,
  macroPromptWithExtra,
  parseScratchMacros,
  serializeScratchMacros,
} from './scratch-macros'

describe('scratch macros', () => {
  it('round-trips through the settings row', () => {
    const macros = [{ id: 'macro-1', name: 'Triage', prompt: 'Look at CI' }]
    expect(parseScratchMacros(serializeScratchMacros(macros))).toEqual(macros)
  })

  it('drops malformed rows', () => {
    expect(parseScratchMacros(undefined)).toEqual([])
    expect(parseScratchMacros('not json')).toEqual([])
    expect(parseScratchMacros('{"id":"x"}')).toEqual([])
    expect(
      parseScratchMacros('[null, {"name":"no id"}, {"id":"a","name":7}]'),
    ).toEqual([{ id: 'a', name: '', prompt: '' }])
  })

  it('needs a name and a prompt to run', () => {
    expect(isRunnableMacro({ id: 'a', name: 'X', prompt: 'Y' })).toBe(true)
    expect(isRunnableMacro({ id: 'a', name: ' ', prompt: 'Y' })).toBe(false)
    expect(isRunnableMacro({ id: 'a', name: 'X', prompt: '' })).toBe(false)
  })

  it('previews the first 20 characters on one line', () => {
    expect(macroPromptPreview('Check CI')).toBe('Check CI')
    expect(macroPromptPreview('Check what failed\nin CI overnight')).toBe(
      'Check what failed in…',
    )
  })

  it('finds a runnable macro by its exact name', () => {
    const macros = [
      { id: 'a', name: 'Morning triage', prompt: 'Check CI' },
      { id: 'b', name: 'Draft', prompt: '' },
    ]
    expect(macroNamed(macros, ' morning TRIAGE ')?.id).toBe('a')
    expect(macroNamed(macros, 'Morning')).toBeUndefined()
    expect(macroNamed(macros, 'Draft')).toBeUndefined()
  })

  it('appends extra text to the prompt', () => {
    expect(macroPromptWithExtra('Check CI ', '')).toBe('Check CI')
    expect(macroPromptWithExtra('Check CI', ' for main ')).toBe(
      'Check CI\n\nfor main',
    )
  })
})
