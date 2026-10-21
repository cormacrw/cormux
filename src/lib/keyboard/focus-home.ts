// The composer is where focus belongs in a conversation. These decide when it can take focus
// back without pulling it away from something the user is in the middle of.

let lastKeyAt = 0

/** Records keyboard use, so focus moved by arrow keys (through tabs, say) stays where it went. */
export function trackKeyboardInput(target: Window) {
  const onKey = () => (lastKeyAt = performance.now())
  target.addEventListener('keydown', onKey, true)
  return () => target.removeEventListener('keydown', onKey, true)
}

/** The user moved focus with the keyboard just now. */
export function keyboardJustMoved(withinMs = 400): boolean {
  return performance.now() - lastKeyAt < withinMs
}

const TEXT_INPUT = /^(text|search|url|email|password|number|tel)$/

/** A field the user types in: a text input, textarea, select or contenteditable. */
export function isEditable(element: Element | null): boolean {
  if (!element) return false
  if (element instanceof HTMLTextAreaElement) return true
  if (element instanceof HTMLSelectElement) return true
  if (element instanceof HTMLInputElement) return TEXT_INPUT.test(element.type)
  return element instanceof HTMLElement && element.isContentEditable
}

/** An open dialog, menu or list popover holds focus for now. */
function overlayOpen(): boolean {
  return !!document.querySelector(
    '[role="dialog"], [role="alertdialog"], [role="menu"][data-state="open"], [role="listbox"]:not(#composer-picker)',
  )
}

/**
 * Whether the composer may take focus now: not while another field has it, an overlay is
 * open, or the user just moved focus somewhere with the keyboard.
 */
export function composerMayTakeFocus(composer: HTMLElement | null): boolean {
  const active = document.activeElement
  if (active === composer) return false
  if (overlayOpen()) return false
  if (active && active !== document.body && isEditable(active)) return false
  if (
    active &&
    active !== document.body &&
    active.closest('[role="tab"], [role="tablist"]') &&
    keyboardJustMoved()
  ) {
    return false
  }
  return true
}
