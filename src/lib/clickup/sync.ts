import type { Window } from '@tauri-apps/api/window'
import { clickup } from './store.svelte'

// Teammates move tasks in ClickUp too, so the sprint refreshes every couple of minutes.
const POLL_MS = 120_000

/**
 * Keeps the sprint current: a refresh whenever the window regains focus, and a poll while it
 * has it (a window in the background skips its turns; focusing it refreshes anyway).
 * Returns the uninstall.
 */
export async function installClickupSync(win: Window): Promise<() => void> {
  const unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
    if (focused) void clickup.refresh()
  })
  const poll = window.setInterval(() => {
    if (document.hasFocus()) void clickup.refresh()
  }, POLL_MS)
  return () => {
    unlistenFocus()
    window.clearInterval(poll)
  }
}
