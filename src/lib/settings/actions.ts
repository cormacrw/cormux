import type { SettingsPatch } from '$lib/ipc/bindings'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { settings } from './store.svelte'

/** Changes settings at once and saves them, toasting if the core refuses. */
export async function updateSettings(patch: SettingsPatch) {
  const error = await settings.update(patch)
  if (error) toastCoreError(error)
}
