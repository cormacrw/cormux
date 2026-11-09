import { reloadEnvironment } from '$lib/settings/actions'
import { events } from '$lib/ipc'
import { runShellCommand } from './shell-commands'

export function bindNativeMenu() {
  return events.menuChosen.listen(({ payload }) => {
    switch (payload.command) {
      case 'openSettings':
        runShellCommand('settings')
        break
      case 'commandPalette':
        runShellCommand('palette')
        break
      case 'newWorkspace':
        runShellCommand('newThread')
        break
      case 'reloadEnvironment':
        void reloadEnvironment()
        break
    }
  })
}
