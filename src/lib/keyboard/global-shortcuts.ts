import { openWorkspace } from '$lib/workspace/open'
import { openSettings } from '$lib/settings/open'
import { navigate } from '$lib/core/nav.svelte'
import { switchConversation } from '$lib/thread/actions'
import {
  closeActiveWorkspacePopover,
  dismissOpenPopover,
} from '$lib/keyboard/popover-registry'
import { nav } from '$lib/core/nav.svelte'
import { palette } from '$lib/state/palette.svelte'
import { threadFind } from '$lib/thread/find-store.svelte'
import { handleSwitcherKeydown } from '$lib/switcher/navigate'
import { threadTimeline } from '$lib/thread/timeline-store.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { sidebarWorkspaces } from '$lib/sidebar/thread-list-store.svelte'
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
      (nav.view === 'workspace' && workspaceUi.activeTab === 'thread') ||
      nav.view === 'scratch'
    if (conversation && !shellDialogs.anyOpen()) {
      event.preventDefault()
      threadFind.request()
    }
    return
  }

  if (isShortcut(event, 'newThread')) {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) navigate({ view: 'newThread' })
    return
  }

  if (isShortcut(event, 'settings')) {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) openSettings()
    return
  }

  if (mod && !event.shiftKey && !event.altKey && /^[1-9]$/.test(event.key)) {
    event.preventDefault()
    const workspace = sidebarWorkspaces.ordered[Number(event.key) - 1]
    if (workspace && !shellDialogs.anyOpen()) openWorkspace(workspace.id)
    return
  }

  if (isShortcut(event, 'toggleGit') && nav.view === 'workspace') {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) workspaceUi.toggleGit()
    return
  }

  if (isShortcut(event, 'toggleOutput') && nav.view === 'workspace') {
    event.preventDefault()
    if (!shellDialogs.anyOpen()) workspaceUi.toggleOutputTab()
    return
  }

  if (
    event.ctrlKey &&
    event.key === '`' &&
    nav.view === 'workspace' &&
    !shellDialogs.anyOpen()
  ) {
    event.preventDefault()
    workspaceUi.toggleOutputTab()
    return
  }

  if (
    isShortcut(event, 'newSession') &&
    nav.view === 'workspace' &&
    workspaceUi.activeTab === 'thread' &&
    nav.threadId &&
    !shellDialogs.anyOpen()
  ) {
    event.preventDefault()
    const threadId = nav.threadId
    if (threadTimeline.threadView(threadId).canClearSession) {
      void switchConversation(threadId, 'new')
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
