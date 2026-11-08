import { openSettings } from '$lib/settings/open'
import { navigate } from '$lib/core/nav.svelte'
import { commands, events } from '$lib/ipc'
import {
  toastCoreError,
  toastEnvironmentReloaded,
} from '$lib/feedback/wire-feedback'
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
        openSettings()
        break
      case 'commandPalette':
        palette.toggle()
        break
      case 'newWorkspace':
        navigate({ view: 'newThread' })
        break
      case 'reloadEnvironment':
        void reloadEnvironment()
        break
    }
  })
}
