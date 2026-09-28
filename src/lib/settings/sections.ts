export const SETTINGS_SECTIONS = [
  { id: 'agents', label: 'Agents' },
  { id: 'repos', label: 'Repos' },
  { id: 'general', label: 'General' },
  { id: 'github', label: 'GitHub' },
  { id: 'notifications', label: 'Notifications' },
  { id: 'skills', label: 'Skills' },
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
