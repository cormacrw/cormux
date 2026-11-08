import type { Window } from '@tauri-apps/api/window'
import { onSnapshot } from '$lib/core/sync'
import { notifyHarness, openNotifiedTarget } from './os-notifications'
import { showToast } from './show-toast'
import { createSupervisor } from './supervisor'

/**
 * Toasts and notifications for what changed while you looked away. Tracks the window's focus,
 * and on regaining it opens what the last notification was about. Returns the uninstall.
 */
export async function installSupervision(win: Window): Promise<() => void> {
  let focused = await win.isFocused()
  const unlistenFocus = await win.onFocusChanged(({ payload }) => {
    focused = payload
    if (payload) openNotifiedTarget()
  })
  const supervise = createSupervisor({
    toast: showToast,
    notify: notifyHarness,
    focused: () => focused,
  })
  const stopWatching = onSnapshot((snapshot) => void supervise(snapshot))
  return () => {
    unlistenFocus()
    stopWatching()
  }
}
