import { SvelteSet } from 'svelte/reactivity'

/**
 * Who has the keyboard: the one place that knows what is open over the page.
 *
 * - A `modal` (a dialog, or a shell dialog on its way) holds everything: shortcuts and the
 *   palette wait for it.
 * - An `overlay` (a menu, a picker, the palette, the media viewer) takes focus and Esc while
 *   it's open, but app shortcuts still work.
 * - A workspace popover is an overlay that Esc closes from anywhere, handing focus back.
 * - With nothing open, Esc goes down the page's own chain: a slide-out closes first, and only
 *   then does a working agent stop.
 */
type LayerKind = 'modal' | 'overlay'

type Layer = { kind: LayerKind; isOpen: () => boolean }

type PopoverHandle = {
  close: () => void
  restoreFocus: () => void
}

const ESCAPE_ORDER = ['slideOut', 'agent'] as const

type EscapeStep = (typeof ESCAPE_ORDER)[number]

/** Returns whether it did something, which ends the chain. */
type EscapeHandler = () => boolean

class KeyboardLayers {
  private layers = new SvelteSet<Layer>()
  private popover: PopoverHandle | null = null
  private underModal = new Set<() => void>()
  private escapes = new Map<EscapeStep, Set<EscapeHandler>>(
    ESCAPE_ORDER.map((step) => [step, new Set()]),
  )

  /**
   * Holds the keyboard while `isOpen()` is true, read live, so a layer that has just closed no
   * longer counts. Returns the release, for an `$effect` or a long-lived store.
   */
  hold(kind: LayerKind, isOpen: () => boolean) {
    const layer = { kind, isOpen }
    this.layers.add(layer)
    return () => void this.layers.delete(layer)
  }

  /** A modal is open: shortcuts and the palette wait. */
  get modal(): boolean {
    for (const layer of this.layers) {
      if (layer.kind === 'modal' && layer.isOpen()) return true
    }
    return false
  }

  /** Anything is open over the page; focus and Esc are its own. */
  get held(): boolean {
    for (const layer of this.layers) if (layer.isOpen()) return true
    return false
  }

  /**
   * Only one popover at a time: the ⋯ menu, the model and effort pickers, the new thread's repo.
   * Claiming one closes the last; Esc, a dialog or the palette closes whichever is open.
   */
  claimPopover(handle: PopoverHandle) {
    if (this.popover && this.popover !== handle) this.popover.close()
    this.popover = handle
  }

  releasePopover(handle: PopoverHandle) {
    if (this.popover === handle) this.popover = null
  }

  closePopover(): boolean {
    if (!this.popover) return false
    this.popover.close()
    this.popover.restoreFocus()
    return true
  }

  /** Closes `close`'s overlay whenever a dialog opens over the page. Returns the release. */
  closeUnderModal(close: () => void) {
    this.underModal.add(close)
    return () => void this.underModal.delete(close)
  }

  /** A dialog is opening: the popover and every overlay that gives way to one close. */
  modalOpening() {
    this.closePopover()
    for (const close of this.underModal) close()
  }

  /** Adds a step to Esc's chain under the page; the order is `ESCAPE_ORDER`, not registration's. */
  onEscape(step: EscapeStep, handle: EscapeHandler) {
    this.escapes.get(step)!.add(handle)
    return () => void this.escapes.get(step)!.delete(handle)
  }

  /** Esc with nothing open: the first step that does something ends it. */
  runEscape(): boolean {
    for (const step of ESCAPE_ORDER) {
      for (const handle of this.escapes.get(step)!) if (handle()) return true
    }
    return false
  }
}

export const keyboardLayers = new KeyboardLayers()
