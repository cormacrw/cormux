import type { Component } from 'svelte'
import type { SettingsSectionId } from '$lib/core/route'
import AgentsSettings from '$lib/settings/components/AgentsSettings.svelte'
import AppearanceSettings from '$lib/settings/components/AppearanceSettings.svelte'
import BangsSettings from '$lib/bangs/BangsSettings.svelte'
import ClickupSettings from '$lib/clickup/components/ClickupSettings.svelte'
import ExperimentalSettings from '$lib/settings/components/ExperimentalSettings.svelte'
import GeneralSettings from '$lib/settings/components/GeneralSettings.svelte'
import GithubSettings from '$lib/settings/components/GithubSettings.svelte'
import NotificationsSettings from '$lib/settings/components/NotificationsSettings.svelte'
import PromptsSettings from '$lib/settings/components/PromptsSettings.svelte'
import ReposSettings from '$lib/settings/components/ReposSettings.svelte'
import ShortcutsSettings from '$lib/settings/components/ShortcutsSettings.svelte'

type SettingsSection = {
  id: SettingsSectionId
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

// Every section a route can name is on the page; this stops compiling when one isn't.
true satisfies [
  Exclude<SettingsSectionId, (typeof SETTINGS_SECTIONS)[number]['id']>,
] extends [never]
  ? true
  : false
