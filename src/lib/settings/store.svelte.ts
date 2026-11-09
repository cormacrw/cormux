import type { Settings, SettingsPatch } from '$lib/ipc/bindings'
import { defaultSettings } from './defaults'
import { commands, type CoreError } from '$lib/ipc'
import { onSnapshot, refreshSnapshot } from '$lib/core/sync'
import { deepEqual } from '$lib/deep-equal'

export class SettingsStore {
  // Raw and replaced whole: readers see a new object only when a setting changed.
  values = $state.raw<Settings>(defaultSettings())

  hydrate(settings: Settings) {
    if (!deepEqual(settings, this.values)) this.values = settings
  }

  /**
   * Saves the change and resolves once a snapshot taken after it has landed, so what the core
   * kept arrives in order with every other change; resolves why it didn't save.
   */
  async update(patch: SettingsPatch): Promise<CoreError | null> {
    const result = await commands.updateSettings(patch)
    if (result.status === 'error') return result.error
    await refreshSnapshot()
    return null
  }
}

export const settings = new SettingsStore()

onSnapshot((snapshot) => settings.hydrate(snapshot.settings))
