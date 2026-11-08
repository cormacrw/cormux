type PopoverHandle = {
  close: () => void
  restoreFocus: () => void
}

let active: PopoverHandle | null = null

/**
 * Only one popover at a time: the ⋯ menu, the model and effort pickers, the new thread's repo.
 * Esc, a dialog or the palette closes whichever is open.
 */
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
