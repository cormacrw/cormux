import type { Component } from 'svelte'
import AgentsSettings from './AgentsSettings.svelte'
import AppearanceSettings from './AppearanceSettings.svelte'
import BangsSettings from './BangsSettings.svelte'
import ClickupSettings from './ClickupSettings.svelte'
import ExperimentalSettings from './ExperimentalSettings.svelte'
import GeneralSettings from './GeneralSettings.svelte'
import GithubSettings from './GithubSettings.svelte'
import NotificationsSettings from './NotificationsSettings.svelte'
import PromptsSettings from './PromptsSettings.svelte'
import ReposSettings from './ReposSettings.svelte'
import ShortcutsSettings from './ShortcutsSettings.svelte'

type SettingsSection = {
  id: string
  label: string
  description: string
  component: Component
  /** `data-od-id` when the panel inside already uses `settings-<id>`. */
  odId?: string
  /** Lists that space their own rows, so they sit right under the heading. */
  flush?: boolean
}

/** Settings, in page and nav order. */
export const SETTINGS_SECTIONS = [
  {
    id: 'appearance',
    label: 'Appearance',
    description: 'Mode, colour theme and text sizes on this Mac.',
    component: AppearanceSettings,
  },
  {
    id: 'general',
    label: 'General',
    description: 'Defaults for new workspaces and how the app behaves.',
    component: GeneralSettings,
  },
  {
    id: 'agents',
    label: 'Agents',
    description: 'Which engine new workspaces use, and whether they ask first.',
    component: AgentsSettings,
  },
  {
    id: 'prompts',
    label: 'Prompts',
    description: 'What Cormux asks agents on your behalf.',
    component: PromptsSettings,
    odId: 'settings-prompts-section',
  },
  {
    id: 'github',
    label: 'GitHub',
    description: 'Pull requests sync through the gh CLI.',
    component: GithubSettings,
    odId: 'settings-github-section',
  },
  {
    id: 'clickup',
    label: 'ClickUp',
    description: "Your team's current sprint as a board.",
    component: ClickupSettings,
    odId: 'settings-clickup-section',
  },
  {
    id: 'repos',
    label: 'Repos',
    description: 'Local folders that new workspaces can check out from.',
    component: ReposSettings,
    flush: true,
  },
  {
    id: 'bangs',
    label: 'Bangs',
    description:
      'Short names for prompts you use a lot. Type !name in any prompt bar to put the prompt in, or in the command palette to start a thread with it.',
    component: BangsSettings,
    flush: true,
  },
  {
    id: 'notifications',
    label: 'Notifications',
    description: 'Desktop alerts while Cormux is in the background.',
    component: NotificationsSettings,
    odId: 'settings-notifications-section',
  },
  {
    id: 'shortcuts',
    label: 'Keyboard shortcuts',
    description: 'Keys for getting around Cormux.',
    component: ShortcutsSettings,
  },
  {
    id: 'experimental',
    label: 'Experimental',
    description: 'Features still taking shape. They may change or go away.',
    component: ExperimentalSettings,
  },
] as const satisfies readonly SettingsSection[]

export type SettingsSectionId = (typeof SETTINGS_SECTIONS)[number]['id']

export const SETTINGS_SECTION_IDS: SettingsSectionId[] = SETTINGS_SECTIONS.map(
  (section) => section.id,
)
