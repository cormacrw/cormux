import type { Settings, SettingsPatch } from '$lib/ipc/bindings'
import { defaultSettings } from './defaults'
import { commands, type CoreError } from '$lib/ipc'
import { onSnapshot } from '$lib/core/sync'
import { deepEqual } from '$lib/deep-equal'

function applied(settings: Settings, patch: SettingsPatch): Settings {
  const changes = Object.fromEntries(
    Object.entries(patch).filter(([, value]) => value != null),
  )
  return { ...settings, ...changes }
}

export class SettingsStore {
  // Raw and replaced whole: readers see a new object only when a setting changed.
  values = $state.raw<Settings>(defaultSettings())

  hydrate(settings: Settings) {
    if (!deepEqual(settings, this.values)) this.values = settings
  }

  /** Shows the change at once, then saves it; the core's answer is what stays. Resolves why it didn't save. */
  async update(patch: SettingsPatch): Promise<CoreError | null> {
    this.values = applied(this.values, patch)
    const result = await commands.updateSettings(patch)
    if (result.status === 'error') return result.error
    this.hydrate(result.data)
    return null
  }
}

export const settings = new SettingsStore()

onSnapshot((snapshot) => settings.hydrate(snapshot.settings))
