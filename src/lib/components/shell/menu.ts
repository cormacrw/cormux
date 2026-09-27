import { listen } from '@tauri-apps/api/event'
import { commands } from '$lib/ipc'
import { app } from '$lib/state'

export async function bindNativeMenu() {
  const unlisteners = await Promise.all([
    listen('menu://open-settings', () => {
      app.openSettings()
    }),
    listen('menu://command-palette', () => {
      window.dispatchEvent(new CustomEvent('cormux:command-palette'))
    }),
    listen('menu://new-workspace', () => {
      window.dispatchEvent(new CustomEvent('cormux:new-workspace'))
    }),
    listen('menu://reload-environment', () => {
      void commands.reloadEnvironment()
    }),
  ])

  return () => {
    for (const unlisten of unlisteners) {
      unlisten()
    }
  }
}
