import type { Action } from 'svelte/action'
import { bangNamed } from '$lib/settings/bangs'
import { settings } from '$lib/state/settings.svelte'

/**
 * For prompt fields without the composer's picker (the New workspace and New
 * scratch dialogs): typing `!name` and then a space swaps in that bang's prompt.
 */
export const bangExpand: Action<HTMLTextAreaElement | HTMLInputElement> = (
  node,
) => {
  function onInput(event: Event) {
    const input = event as InputEvent
    if (input.inputType !== 'insertText' || input.data !== ' ') return
    const caret = node.selectionStart ?? 0
    const before = node.value.slice(0, caret - 1)
    const match = /(^|\s)!([\w-]+)$/.exec(before)
    if (!match) return
    const bang = bangNamed(settings.bangs, match[2]!)
    if (!bang) return
    const start = match.index + (match[1]?.length ?? 0)
    // Replaces `!name ` and keeps undo, then tells bound values it changed.
    node.setRangeText(`${bang.prompt.trim()} `, start, caret, 'end')
    node.dispatchEvent(new Event('input', { bubbles: true }))
  }
  node.addEventListener('input', onInput)
  return {
    destroy: () => node.removeEventListener('input', onInput),
  }
}
