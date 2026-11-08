import type { Settings, SettingsPatch } from '$lib/ipc/bindings'
import { defaultSettings } from './defaults'
import { commands, type CoreError } from '$lib/ipc'
import { onSnapshot } from '$lib/core/sync'
import { deepEqual } from '$lib/deep-equal'
import { latestRequests } from '$lib/core/owner-scoped'

function patchedKeys(patch: SettingsPatch): (keyof Settings)[] {
  return Object.entries(patch)
    .filter(([, value]) => value != null)
    .map(([key]) => key as keyof Settings)
}

function applied(settings: Settings, patch: SettingsPatch): Settings {
  return {
    ...settings,
    ...Object.fromEntries(
      patchedKeys(patch).map((key) => [key, patch[key as keyof SettingsPatch]]),
    ),
  }
}

export class SettingsStore {
  // Raw and replaced whole: readers see a new object only when a setting changed.
  values = $state.raw<Settings>(defaultSettings())

  hydrate(settings: Settings) {
    if (!deepEqual(settings, this.values)) this.values = settings
  }

  private readonly saves = latestRequests<keyof Settings | 'any'>()

  /**
   * Shows the change at once, then saves it; the latest save's answer from the core is what stays,
   * and a refused one is taken back. Resolves why it didn't save.
   */
  async update(patch: SettingsPatch): Promise<CoreError | null> {
    const before = this.values
    const keys = patchedKeys(patch)
    const isLatest = this.saves('any')
    const keyIsLatest = keys.map((key) => [key, this.saves(key)] as const)
    this.values = applied(this.values, patch)
    const result = await commands.updateSettings(patch)
    if (result.status === 'error') {
      // A later change to the same setting stands; it has its own save.
      const restored = keyIsLatest
        .filter(([, latest]) => latest())
        .map(([key]) => [key, before[key]])
      if (restored.length)
        this.values = { ...this.values, ...Object.fromEntries(restored) }
      return result.error
    }
    if (isLatest()) this.hydrate(result.data)
    return null
  }
}

export const settings = new SettingsStore()

onSnapshot((snapshot) => settings.hydrate(snapshot.settings))
