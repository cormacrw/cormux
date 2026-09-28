import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'

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

export async function notifyHarness(title: string, body: string) {
  if (!(await ensurePermission())) return
  try {
    sendNotification({ title, body })
  } catch {
    // Ignore when notifications are unavailable.
  }
}
