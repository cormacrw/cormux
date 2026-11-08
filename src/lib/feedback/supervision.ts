import type { Window } from '@tauri-apps/api/window'
import { events } from '$lib/ipc'
import { onSnapshot } from '$lib/core/sync'
import { notifyOs, openNotifiedTarget } from './os-notifications'
import { createSupervisor } from './supervisor'
import { coreToastTarget } from './wire-feedback'

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
    notify: notifyOs,
    focused: () => focused,
  })
  const stopWatching = onSnapshot((snapshot) => void supervise(snapshot))
  // The core says which of its toasts are worth a notification; this says whether you're away.
  const unlistenToasts = await events.toastRaised.listen(({ payload }) => {
    const { notification } = payload.payload
    if (notification && !focused) {
      void notifyOs(notification, coreToastTarget(payload.payload))
    }
  })
  return () => {
    unlistenFocus()
    stopWatching()
    unlistenToasts()
  }
}
