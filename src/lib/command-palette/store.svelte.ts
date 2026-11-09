import { keyboardLayers } from '$lib/keyboard/layers.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'

/** Whether the command palette is open (⌘K, the menu bar and the sidebar open it). */
class PaletteStore {
  isOpen = $state(false)

  constructor() {
    // A dialog opening from a shortcut takes over from the palette.
    keyboardLayers.closeUnderModal(() => this.close())
  }

  open() {
    if (shellDialogs.anyOpen()) return
    keyboardLayers.closePopover()
    this.isOpen = true
  }

  close() {
    this.isOpen = false
  }

  toggle() {
    if (this.isOpen) this.close()
    else this.open()
  }
}

export const palette = new PaletteStore()
