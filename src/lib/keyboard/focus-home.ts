import { keyboardLayers } from './layers.svelte'

// The composer is where focus belongs in a conversation. These decide when it can take focus
// back without pulling it away from something the user is in the middle of.

let lastKeyAt = 0

/** Records keyboard use, so focus moved by arrow keys (through tabs, say) stays where it went. */
export function trackKeyboardInput(target: Window) {
  const onKey = () => (lastKeyAt = performance.now())
  target.addEventListener('keydown', onKey, true)
  return () => target.removeEventListener('keydown', onKey, true)
}

function keyboardJustMoved(withinMs = 400): boolean {
  return performance.now() - lastKeyAt < withinMs
}

const TEXT_INPUT = /^(text|search|url|email|password|number|tel)$/

export function isEditable(element: Element | null): boolean {
  if (!element) return false
  if (element instanceof HTMLTextAreaElement) return true
  if (element instanceof HTMLSelectElement) return true
  if (element instanceof HTMLInputElement) return TEXT_INPUT.test(element.type)
  return element instanceof HTMLElement && element.isContentEditable
}

const OVERLAY_ROLES = ['dialog', 'alertdialog', 'menu', 'listbox']

/** A selector for any overlay, each role narrowed by `state` (`[data-state="open"]`, say). */
export function overlaySelector(state = ''): string {
  return OVERLAY_ROLES.map((role) => `[role="${role}"]${state}`).join(', ')
}

/** The prompts mounted, the one on screen last. */
const prompts: (() => HTMLElement | null | undefined)[] = []

/** For a composer: makes its input the prompt `focusPrompt` reaches. Returns the release. */
export function registerPrompt(input: () => HTMLElement | null | undefined) {
  prompts.push(input)
  return () => {
    const index = prompts.lastIndexOf(input)
    if (index !== -1) prompts.splice(index, 1)
  }
}

/** Puts focus in the prompt on screen, as asked; false when there isn't one. */
export function focusPrompt(): boolean {
  const prompt = prompts.at(-1)?.()
  prompt?.focus()
  return prompt != null
}

/**
 * For a composer picker that has finished closing: back to the prompt, unless the next picker
 * opened meanwhile, since one shortcut can swap one picker for another.
 */
export function focusPromptAfterPicker() {
  if (!keyboardLayers.held) focusPrompt()
}

export function composerMayTakeFocus(composer: HTMLElement | null): boolean {
  const active = document.activeElement
  if (active === composer) return false
  if (keyboardLayers.held) return false
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
