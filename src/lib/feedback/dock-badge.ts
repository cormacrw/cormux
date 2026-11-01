import { getCurrentWindow } from '@tauri-apps/api/window'
import { dockBadgeCount } from './toast-payload'

let shown: number | undefined | null = null

/** Sets the dock badge, skipping the round trip when it already shows this count. */
export async function syncDockBadge(pendingApprovals: number) {
  const count = dockBadgeCount(pendingApprovals)
  if (count === shown) return
  shown = count
  try {
    await getCurrentWindow().setBadgeCount(count)
  } catch {
    // Not running inside Tauri (e.g. vitest / vite-only).
    shown = null
  }
}
