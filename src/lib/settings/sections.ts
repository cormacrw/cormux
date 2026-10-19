export const SETTINGS_SECTIONS = [
  { id: 'appearance', label: 'Appearance' },
  { id: 'general', label: 'General' },
  { id: 'agents', label: 'Agents' },
  { id: 'github', label: 'GitHub' },
  { id: 'clickup', label: 'ClickUp' },
  { id: 'repos', label: 'Repos' },
  { id: 'macros', label: 'Scratch macros' },
  { id: 'notifications', label: 'Notifications' },
  { id: 'shortcuts', label: 'Keyboard shortcuts' },
] as const

export type SettingsSectionId = (typeof SETTINGS_SECTIONS)[number]['id']

export function normalizeSettingsSection(section: string): SettingsSectionId {
  if (section === 'engines') return 'agents'
  if (SETTINGS_SECTIONS.some((row) => row.id === section)) {
    return section as SettingsSectionId
  }
  return 'agents'
}

export function settingsSectionDomId(section: SettingsSectionId) {
  return `settings-${section}`
}
