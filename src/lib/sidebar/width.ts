import { readPref, writePref } from '$lib/primitives/local-pref'

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
  const raw = readPref(SIDEBAR_WIDTH_KEY)
  const width = Number(raw)
  return raw && Number.isFinite(width) ? clampSidebarWidth(width) : null
}

export function writeSidebarWidth(width: number | null) {
  writePref(SIDEBAR_WIDTH_KEY, width == null ? null : String(width))
}
