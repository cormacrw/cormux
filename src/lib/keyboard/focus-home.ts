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
function keyboardJustMoved(withinMs = 400): boolean {
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

/** The roles a dialog, menu or list renders while it's up. */
const OVERLAY_ROLES = ['dialog', 'alertdialog', 'menu', 'listbox']

/** A selector for any overlay, each role narrowed by `state` (`[data-state="open"]`, say). */
export function overlaySelector(state = ''): string {
  return OVERLAY_ROLES.map((role) => `[role="${role}"]${state}`).join(', ')
}

// A layer on its way out (`data-state="closed"`) no longer counts. The composer's own
// picker isn't an overlay: it sits in the prompt and handles its keys itself.
const OPEN_OVERLAY = overlaySelector(
  ':not([data-state="closed"]):not(#composer-picker)',
)

/** An open dialog, menu or list holds the keyboard for now: focus and Esc are its own. */
export function overlayOpen(): boolean {
  return !!document.querySelector(OPEN_OVERLAY)
}

/** The composer's textarea. */
export const PROMPT_ID = 'composer-input'

/** Puts focus in the conversation's prompt; false when none is on screen. */
export function focusPrompt(): boolean {
  const prompt = document.getElementById(PROMPT_ID)
  prompt?.focus()
  return prompt != null
}

/**
 * For a composer picker that has finished closing: back to the prompt, unless the next picker
 * opened meanwhile, since one shortcut can swap one picker for another.
 */
export function focusPromptAfterPicker() {
  const nextPicker = document.querySelector(
    '[role="menu"][data-state="open"], [data-slot="select-content"][data-state="open"]',
  )
  if (!nextPicker) focusPrompt()
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
