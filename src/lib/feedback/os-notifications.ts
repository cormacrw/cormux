import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'
import {
  hasToastTarget,
  openToastTarget,
  type ToastTarget,
} from './toast-target'

let permissionChecked = false

async function ensurePermission() {
  if (permissionChecked) return isPermissionGranted()
  permissionChecked = true
  let granted = await isPermissionGranted()
  if (!granted) {
    const result = await requestPermission()
    granted = result === 'granted'
  }
  return granted
}

/**
 * Where the latest notification points. The notification plugin can't report a click on
 * macOS, but clicking one brings Cormux to the front, so the next focus opens this.
 */
let pendingTarget: ToastTarget | null = null

export async function notifyHarness(
  title: string,
  body: string,
  target?: ToastTarget,
) {
  if (!(await ensurePermission())) return
  try {
    sendNotification({ title, body })
    pendingTarget = target && hasToastTarget(target) ? target : null
  } catch {
    // Ignore when notifications are unavailable.
  }
}

/** Call when the window gains focus: opens what the last notification was about, once. */
export function openNotifiedTarget() {
  const target = pendingTarget
  pendingTarget = null
  if (target) openToastTarget(target)
}
