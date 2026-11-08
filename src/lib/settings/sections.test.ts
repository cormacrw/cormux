import { describe, expect, it } from 'vitest'
import { normalizeSettingsSection } from './sections'

describe('normalizeSettingsSection', () => {
  it('maps legacy engines section to agents', () => {
    expect(normalizeSettingsSection('engines')).toBe('agents')
    expect(normalizeSettingsSection('repos')).toBe('repos')
  })
})
