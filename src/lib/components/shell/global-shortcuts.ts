// The app's keyboard router, beside `menu.ts` (the menu bar's). Buttons on screen handle
// their own shortcuts through `shellDialogs.allows`; this handles the ones with no button.
import { openWorkspace } from '$lib/workspace/open'
import { openSettings } from '$lib/settings/open'
import { nav, navigate } from '$lib/core/nav.svelte'
import { switchConversation } from '$lib/thread/actions'
import {
  closeActiveWorkspacePopover,
  dismissOpenPopover,
} from '$lib/keyboard/popover-registry'
import { focusPrompt, overlayOpen } from '$lib/keyboard/focus-home'
import { runEscape } from '$lib/keyboard/escape'
import { palette } from '$lib/command-palette/store.svelte'
import { threadFind } from '$lib/thread/find-store.svelte'
import { handleSwitcherKeydown } from '$lib/switcher/navigate'
import { threadTimeline } from '$lib/thread/timeline-store.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { sidebarWorkspaces } from '$lib/sidebar/thread-list-store.svelte'
import { isShortcut } from '$lib/keyboard/shortcuts'

export function handleGlobalKeydown(event: KeyboardEvent) {
  if (handleSwitcherKeydown(event)) return

  if (isShortcut(event, 'palette')) {
    event.preventDefault()
    palette.toggle()
    return
  }

  if (isShortcut(event, 'save')) {
    event.preventDefault()
    return
  }

  if (event.key === 'Escape') {
    onEscape(event)
    return
  }

  // While a dialog is open the rest wait; each still keeps its key from the webview.
  const gated = shellDialogs.anyOpen()
  const inWorkspace = nav.view === 'workspace'
  const onThread = inWorkspace && workspaceUi.activeTab === 'thread'

  // Find in the conversation on screen: a workspace thread or a scratch.
  if (isShortcut(event, 'find')) {
    if ((onThread || nav.view === 'scratch') && !gated) {
      event.preventDefault()
      threadFind.request()
    }
    return
  }

  if (isShortcut(event, 'newThread')) {
    event.preventDefault()
    if (!gated) navigate({ view: 'newThread' })
    return
  }

  if (isShortcut(event, 'settings')) {
    event.preventDefault()
    if (!gated) openSettings()
    return
  }

  if (isShortcut(event, 'openThread')) {
    event.preventDefault()
    const workspace = sidebarWorkspaces.ordered[Number(event.key) - 1]
    if (workspace && !gated) openWorkspace(workspace.id)
    return
  }

  if (isShortcut(event, 'toggleGit') && inWorkspace) {
    event.preventDefault()
    if (!gated) workspaceUi.toggleTray('changes')
    return
  }

  if (isShortcut(event, 'toggleOutput') && inWorkspace) {
    event.preventDefault()
    if (!gated) workspaceUi.toggleTray('output')
    return
  }

  if (isShortcut(event, 'newSession') && onThread && nav.threadId && !gated) {
    event.preventDefault()
    const threadId = nav.threadId
    if (threadTimeline.threadView(threadId).canClearSession) {
      void switchConversation(threadId, 'new')
    }
    return
  }

  // Dialogs and the diff comment box send their own ⌘↵ first.
  if (isShortcut(event, 'composer') && !event.defaultPrevented && !gated) {
    event.preventDefault()
    focusPrompt()
  }
}

/** Esc closes the innermost thing: a popover, then a slide-out, then stops a working agent. */
function onEscape(event: KeyboardEvent) {
  if (closeActiveWorkspacePopover() || dismissOpenPopover()) {
    event.preventDefault()
    return
  }
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return
  // A field, dialog or menu that took Esc has used it.
  if (event.defaultPrevented || overlayOpen() || shellDialogs.anyOpen()) return
  if (runEscape()) event.preventDefault()
}
