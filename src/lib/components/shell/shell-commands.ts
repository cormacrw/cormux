import { openSettings } from '$lib/settings/open'
import { navigate } from '$lib/core/nav.svelte'
import { palette } from '$lib/command-palette/store.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'

export type ShellCommand = 'newThread' | 'settings' | 'palette'

/**
 * The commands both the keyboard and the menu bar reach, so whichever of them gets ⌘N, ⌘, or
 * ⌘K, the open-dialog gate is the same.
 */
export function runShellCommand(command: ShellCommand) {
  if (command === 'palette') {
    palette.toggle()
    return
  }
  if (shellDialogs.anyOpen()) return
  if (command === 'newThread') navigate({ view: 'newThread' })
  else openSettings()
}
