import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { isShortcut, type MatchedShortcutId } from './shortcuts'

/** A shortcut for a button on screen: not held down, and not while a dialog is open. */
export function isHeaderShortcut(event: KeyboardEvent, id: MatchedShortcutId) {
  return isShortcut(event, id) && !event.repeat && !shellDialogs.anyOpen()
}
