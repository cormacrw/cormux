export const SIDEBAR_MIN_WIDTH = 180
export const SIDEBAR_MAX_WIDTH = 440

const SIDEBAR_WIDTH_KEY = 'cormux-sidebar-width'

export function clampSidebarWidth(width: number) {
  return Math.round(
    Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, width)),
  )
}

/** The width the user dragged the sidebar to, or null to keep the CSS default. */
export function readSidebarWidth(): number | null {
  let raw: string | null
  try {
    raw = localStorage.getItem(SIDEBAR_WIDTH_KEY)
  } catch {
    return null
  }
  const width = Number(raw)
  return raw && Number.isFinite(width) ? clampSidebarWidth(width) : null
}

export function writeSidebarWidth(width: number | null) {
  try {
    if (width == null) localStorage.removeItem(SIDEBAR_WIDTH_KEY)
    else localStorage.setItem(SIDEBAR_WIDTH_KEY, String(width))
  } catch {
    // Storage can be unavailable; the width still applies for this session.
  }
}
