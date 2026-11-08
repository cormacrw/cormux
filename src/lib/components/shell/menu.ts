import { commands, events } from '$lib/ipc'
import {
  toastCoreError,
  toastEnvironmentReloaded,
} from '$lib/feedback/wire-feedback'
import { app } from '$lib/state'
import { palette } from '$lib/state/palette.svelte'

async function reloadEnvironment() {
  const result = await commands.reloadEnvironment()
  if (result.status === 'ok') {
    toastEnvironmentReloaded()
    return
  }
  toastCoreError(result.error)
}

export function bindNativeMenu() {
  return events.menuChosen.listen(({ payload }) => {
    switch (payload.command) {
      case 'openSettings':
        app.openSettings()
        break
      case 'commandPalette':
        palette.toggle()
        break
      case 'newWorkspace':
        app.openNewThread()
        break
      case 'reloadEnvironment':
        void reloadEnvironment()
        break
    }
  })
}
