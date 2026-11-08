import { commands, type SettingsPatch } from '$lib/ipc/bindings'
import {
  toastCoreError,
  toastEnvironmentReloaded,
} from '$lib/feedback/wire-feedback'
import { settings } from './store.svelte'

/** Saves settings, toasting if the core refuses. */
export async function updateSettings(patch: SettingsPatch) {
  const error = await settings.update(patch)
  if (error) toastCoreError(error)
}

/** Re-reads the login shell's environment (from the menu bar or Settings), and says so. */
export async function reloadEnvironment() {
  const result = await commands.reloadEnvironment()
  if (result.status === 'ok') toastEnvironmentReloaded()
  else toastCoreError(result.error)
}
