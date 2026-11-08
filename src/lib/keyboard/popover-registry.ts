type PopoverHandle = {
  close: () => void
  restoreFocus: () => void
}

let active: PopoverHandle | null = null

/** Only one workspace popover (⋯, branch picker, etc.) at a time. */
export function claimWorkspacePopover(handle: PopoverHandle) {
  if (active && active !== handle) {
    active.close()
  }
  active = handle
}

export function releaseWorkspacePopover(handle: PopoverHandle) {
  if (active === handle) active = null
}

export function closeActiveWorkspacePopover(): boolean {
  if (!active) return false
  active.close()
  active.restoreFocus()
  return true
}

type PopoverCloser = () => boolean

let closeOpenPopover: PopoverCloser | null = null

/** The popover Esc closes before anything else; it resolves to whether one was open. */
export function registerPopoverCloser(closer: PopoverCloser | null) {
  closeOpenPopover = closer
}

export function dismissOpenPopover(): boolean {
  return closeOpenPopover?.() ?? false
}
