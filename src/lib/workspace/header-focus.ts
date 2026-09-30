import { tick } from 'svelte'

let lastFocusKey: string | null = null

export function rememberHeaderFocusKey(key: string) {
  lastFocusKey = key
}

export function clearHeaderFocusKey() {
  lastFocusKey = null
}

/** After a header rebuild, restore focus to the same control when it still exists. */
export async function restoreHeaderFocus() {
  const key = lastFocusKey
  if (!key) return
  await tick()
  const el = document.querySelector<HTMLElement>(`[data-ws-focus="${key}"]`)
  el?.focus()
}
