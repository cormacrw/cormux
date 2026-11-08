import {
  closeActiveWorkspacePopover,
  dismissOpenPopover,
} from '$lib/keyboard/popover-registry'
import { app } from '$lib/state/app.svelte'
import { palette } from '$lib/state/palette.svelte'
import { threadFind } from '$lib/state/thread-find.svelte'
import { handleSwitcherKeydown } from '$lib/switcher/navigate'
import { threadTimeline } from '$lib/state/thread-timeline.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { sidebarWorkspaces } from '$lib/state/sidebar-workspaces.svelte'
import { isShortcut } from './shortcuts'

export function handleGlobalKeydown(event: KeyboardEvent) {
  if (handleSwitcherKeydown(event)) return
  const mod = event.metaKey || event.ctrlKey

  if (isShortcut(event, 'palette')) {
    event.preventDefault()
    palette.toggle()
    return
  }

  // Keeps the browser save dialog away.
  if (
    mod &&
    event.key.toLowerCase() === 's' &&
    !event.shiftKey &&
    !event.altKey
  ) {
    event.preventDefault()
    return
  }

  // Find in the conversation on screen: a workspace thread or a scratch.
  if (isShortcut(event, 'find')) {
    const conversation =
      (app.view === 'workspace' && workspaceUi.activeTab === 'thread') ||
      app.view === 'scratch'
    if (conversation && !shellDialogs.anyOpen()) {
      event.preventDefault()
      threadFind.request()
    }
    return
  }

  if (isShortcut(event, 'newThread')) {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) app.openNewThread()
    return
  }

  if (isShortcut(event, 'settings')) {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) app.openSettings()
    return
  }

  if (mod && !event.shiftKey && !event.altKey && /^[1-9]$/.test(event.key)) {
    event.preventDefault()
    const workspace = sidebarWorkspaces.ordered[Number(event.key) - 1]
    if (workspace && !shellDialogs.anyOpen()) app.openWorkspace(workspace.id)
    return
  }

  if (isShortcut(event, 'toggleGit') && app.view === 'workspace') {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) workspaceUi.toggleGit()
    return
  }

  if (isShortcut(event, 'toggleOutput') && app.view === 'workspace') {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) workspaceUi.toggleOutputTab()
    return
  }

  if (
    event.ctrlKey &&
    event.key === '`' &&
    app.view === 'workspace' &&
    !shellDialogs.anyOpen()
  ) {
    event.preventDefault()
    workspaceUi.toggleOutputTab()
    return
  }

  if (
    isShortcut(event, 'newSession') &&
    app.view === 'workspace' &&
    workspaceUi.activeTab === 'thread' &&
    app.threadId &&
    !shellDialogs.anyOpen()
  ) {
    event.preventDefault()
    const threadId = app.threadId
    if (threadTimeline.threadView(threadId).canClearSession) {
      void threadTimeline.switchConversation(threadId, 'new')
    }
    return
  }

  if (event.key === 'Escape') {
    if (closeActiveWorkspacePopover()) {
      event.preventDefault()
      return
    }
    if (dismissOpenPopover()) event.preventDefault()
  }
}
