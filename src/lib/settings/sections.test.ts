import { describe, expect, it } from 'vitest'
import { normalizeSettingsSection } from './sections'

const ids = ['agents', 'bangs', 'repos'] as const

describe('normalizeSettingsSection', () => {
  it('maps renamed sections to their new ids', () => {
    expect(normalizeSettingsSection('engines', ids, 'agents')).toBe('agents')
    expect(normalizeSettingsSection('macros', ids, 'agents')).toBe('bangs')
    expect(normalizeSettingsSection('repos', ids, 'agents')).toBe('repos')
  })

  it('falls back for a section that is gone', () => {
    expect(normalizeSettingsSection('homebase', ids, 'agents')).toBe('agents')
  })
})
