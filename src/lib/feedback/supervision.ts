import type { Window } from '@tauri-apps/api/window'
import { events } from '$lib/ipc'
import { notifyOs, openNotifiedTarget } from './os-notifications'
import { coreToastTarget } from './wire-feedback'

/**
 * Desktop notifications for what the core says is worth one. The core decides what and when,
 * honouring the user's settings; this says whether you're away. Tracks the window's focus, and
 * on regaining it opens what the last notification was about. Returns the uninstall.
 */
export async function installSupervision(win: Window): Promise<() => void> {
  let focused = await win.isFocused()
  const unlistenFocus = await win.onFocusChanged(({ payload }) => {
    focused = payload
    if (payload) openNotifiedTarget()
  })
  const unlistenNotifications = await events.notificationRaised.listen(
    ({ payload }) => {
      if (focused) return
      void notifyOs(payload.payload.body, coreToastTarget(payload.payload))
    },
  )
  return () => {
    unlistenFocus()
    unlistenNotifications()
  }
}
