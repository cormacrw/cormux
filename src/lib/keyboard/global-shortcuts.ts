import { closeActiveWorkspacePopover } from '$lib/keyboard/popover-registry'
import { app } from '$lib/state/app.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'

type PopoverCloser = () => boolean

let closeOpenPopover: PopoverCloser | null = null

/** COR-12+ registers the active popover close handler. */
export function registerPopoverCloser(closer: PopoverCloser | null) {
  closeOpenPopover = closer
}

export function dismissOpenPopover() {
  closeOpenPopover?.()
}

function dialogBlocksShortcuts() {
  return shellDialogs.blocksCommandPalette()
}

export function handleGlobalKeydown(event: KeyboardEvent) {
  const mod = event.metaKey || event.ctrlKey
  const key = event.key.toLowerCase()

  if (mod && key === 'k') {
    event.preventDefault()
    app.requestCommandPalette()
    return
  }

  // Global on purpose: fires inside fields and dialogs, and keeps the browser save dialog away.
  if (mod && key === 's' && !event.shiftKey && !event.altKey) {
    event.preventDefault()
    app.requestNewScratch()
    return
  }

  if (mod && key === 'n') {
    event.preventDefault()
    app.requestNewWorkspace()
    return
  }

  if (
    event.ctrlKey &&
    event.key === '`' &&
    app.view === 'workspace' &&
    !dialogBlocksShortcuts()
  ) {
    event.preventDefault()
    workspaceUi.toggleOutputTab()
    return
  }

  if (event.key === 'Escape') {
    if (closeActiveWorkspacePopover()) {
      event.preventDefault()
      return
    }
    if (closeOpenPopover?.()) {
      event.preventDefault()
      return
    }
    if (
      app.view === 'workspace' &&
      workspaceUi.changesOpen &&
      !dialogBlocksShortcuts()
    ) {
      event.preventDefault()
      workspaceUi.changesOpen = false
    }
  }
}
