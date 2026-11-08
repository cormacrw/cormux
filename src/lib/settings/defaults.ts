import { DEFAULT_SETTINGS, type Settings } from '$lib/ipc/bindings'

/** The core's defaults (`Settings::default()`), as an object of their own. */
export function defaultSettings(): Settings {
  return {
    ...DEFAULT_SETTINGS,
    bangs: [...DEFAULT_SETTINGS.bangs],
    clickupHiddenStatuses: [...DEFAULT_SETTINGS.clickupHiddenStatuses],
  }
}
