import { shellDialogs } from '$lib/state/shell-dialogs.svelte'

/** ⌘<key> with no Shift or Option, while no dialog is open. For header buttons. */
export function isHeaderShortcut(event: KeyboardEvent, key: string) {
  return !event.shiftKey && isModShortcut(event, key)
}

/** ⌘⇧<key> with no Option, while no dialog is open. For the composer's pickers. */
export function isPickerShortcut(event: KeyboardEvent, key: string) {
  return event.shiftKey && isModShortcut(event, key)
}

function isModShortcut(event: KeyboardEvent, key: string) {
  return (
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.repeat &&
    event.key.toLowerCase() === key &&
    !shellDialogs.blocksCommandPalette()
  )
}
