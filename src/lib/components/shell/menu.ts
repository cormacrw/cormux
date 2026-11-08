import { listen } from '@tauri-apps/api/event'
import { commands } from '$lib/ipc'
import {
  toastCoreError,
  toastEnvironmentReloaded,
} from '$lib/feedback/wire-feedback'
import { app } from '$lib/state'
import { palette } from '$lib/state/palette.svelte'

export async function bindNativeMenu() {
  const unlisteners = await Promise.all([
    listen('menu://open-settings', () => {
      app.openSettings()
    }),
    listen('menu://command-palette', () => {
      palette.toggle()
    }),
    listen('menu://new-workspace', () => {
      app.openNewThread()
    }),
    listen('menu://reload-environment', () => {
      void (async () => {
        const result = await commands.reloadEnvironment()
        if (result.status === 'ok') {
          toastEnvironmentReloaded()
          return
        }
        toastCoreError(result.error)
      })()
    }),
  ])

  return () => {
    for (const unlisten of unlisteners) {
      unlisten()
    }
  }
}
