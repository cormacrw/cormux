import { openSettings } from '$lib/settings/open'
import { reloadEnvironment } from '$lib/settings/actions'
import { navigate } from '$lib/core/nav.svelte'
import { events } from '$lib/ipc'
import { palette } from '$lib/command-palette/store.svelte'

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
