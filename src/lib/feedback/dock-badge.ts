import { getCurrentWindow } from '@tauri-apps/api/window'
import { dockBadgeCount } from './toast-payload'

export async function syncDockBadge(pendingApprovals: number) {
  const count = dockBadgeCount(pendingApprovals)
  try {
    await getCurrentWindow().setBadgeCount(count)
  } catch {
    // Not running inside Tauri (e.g. vitest / vite-only).
  }
}
