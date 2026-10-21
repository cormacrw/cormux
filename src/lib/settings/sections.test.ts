import { describe, expect, it } from 'vitest'
import { normalizeSettingsSection } from './sections'

describe('normalizeSettingsSection', () => {
  it('maps legacy engines section to agents', () => {
    expect(normalizeSettingsSection('engines')).toBe('agents')
    expect(normalizeSettingsSection('repos')).toBe('repos')
  })
})

describe('shortcut reference', () => {
  it('lists each shortcut once, since Settings keys the list by it', async () => {
    const { SETTINGS_SHORTCUTS } = await import('./shortcuts')
    const keys = SETTINGS_SHORTCUTS.map((shortcut) => shortcut.keys)
    expect(new Set(keys).size).toBe(keys.length)
  })
})
