import { closeActiveWorkspacePopover } from '$lib/keyboard/popover-registry'
import { app } from '$lib/state/app.svelte'
import { threadFind } from '$lib/state/thread-find.svelte'
import { handleSwitcherKeydown } from '$lib/switcher/navigate'
import { threadTimeline } from '$lib/state/thread-timeline.svelte'
import {
  hasSessionToClear,
  isNewSessionShortcut,
} from '$lib/thread/new-session'
import { startNewSession } from '$lib/thread/start-new-session'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { sidebarWorkspaces } from '$lib/state/sidebar-workspaces.svelte'

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
  if (handleSwitcherKeydown(event)) return
  const mod = event.metaKey || event.ctrlKey
  const key = event.key.toLowerCase()

  if (mod && key === 'k') {
    event.preventDefault()
    app.requestCommandPalette()
    return
  }

  // Keeps the browser save dialog away.
  if (mod && key === 's' && !event.shiftKey && !event.altKey) {
    event.preventDefault()
    return
  }

  // Find in the conversation on screen: a workspace thread or a scratch.
  if (mod && key === 'f' && !event.shiftKey && !event.altKey) {
    const conversation =
      (app.view === 'workspace' && workspaceUi.activeTab === 'thread') ||
      app.view === 'scratch'
    if (conversation && !dialogBlocksShortcuts()) {
      event.preventDefault()
      threadFind.request()
    }
    return
  }

  if (mod && key === 'n') {
    event.preventDefault()
    if (!dialogBlocksShortcuts()) app.openNewThread()
    return
  }

  if (mod && key === ',' && !event.shiftKey && !event.altKey) {
    event.preventDefault()
    if (!dialogBlocksShortcuts()) app.openSettings()
    return
  }

  // ⌘1–⌘9 open workspaces in sidebar order.
  if (mod && !event.shiftKey && !event.altKey && /^[1-9]$/.test(event.key)) {
    event.preventDefault()
    const workspace = sidebarWorkspaces.ordered[Number(event.key) - 1]
    if (workspace && !dialogBlocksShortcuts()) app.openWorkspace(workspace.id)
    return
  }

  if (
    mod &&
    key === 'g' &&
    !event.shiftKey &&
    !event.altKey &&
    app.view === 'workspace'
  ) {
    event.preventDefault()
    if (!dialogBlocksShortcuts()) workspaceUi.toggleGit()
    return
  }

  if (
    mod &&
    key === 'o' &&
    !event.shiftKey &&
    !event.altKey &&
    app.view === 'workspace'
  ) {
    event.preventDefault()
    if (!dialogBlocksShortcuts()) workspaceUi.toggleOutputTab()
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

  if (
    isNewSessionShortcut(event) &&
    app.view === 'workspace' &&
    workspaceUi.activeTab === 'thread' &&
    app.threadId &&
    !dialogBlocksShortcuts()
  ) {
    event.preventDefault()
    const threadId = app.threadId
    if (hasSessionToClear(threadTimeline.eventsByThread[threadId] ?? [])) {
      void startNewSession(threadId)
    }
    return
  }

  if (event.key === 'Escape') {
    if (closeActiveWorkspacePopover()) {
      event.preventDefault()
      return
    }
    if (closeOpenPopover?.()) {
      event.preventDefault()
    }
  }
}
