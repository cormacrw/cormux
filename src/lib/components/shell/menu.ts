import { listen } from '@tauri-apps/api/event'
import { commands } from '$lib/ipc'
import {
  toastCoreError,
  toastEnvironmentReloaded,
} from '$lib/feedback/wire-feedback'
import { app } from '$lib/state'

export async function bindNativeMenu() {
  const unlisteners = await Promise.all([
    listen('menu://open-settings', () => {
      app.openSettings()
    }),
    listen('menu://command-palette', () => {
      app.requestCommandPalette()
    }),
    listen('menu://new-workspace', () => {
      app.requestNewWorkspace()
    }),
    listen('menu://new-scratch', () => {
      app.requestNewScratch()
    }),
    listen('menu://reload-environment', () => {
      void (async () => {
        const result = await commands.reloadEnvironment()
        if (result.status === 'ok') {
          toastEnvironmentReloaded()
          return
        }
        toastCoreError(JSON.stringify(result.error))
      })()
    }),
  ])

  return () => {
    for (const unlisten of unlisteners) {
      unlisten()
    }
  }
}
