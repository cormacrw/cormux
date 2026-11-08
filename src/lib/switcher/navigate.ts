import { openWorkspace } from '$lib/nav/open'
import { nav, navigate } from '$lib/core/nav.svelte'
import { clickup } from '$lib/clickup/store.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { switcher } from '$lib/switcher/store.svelte'
import { threads } from '$lib/state/threads.svelte'
import { sidebarWorkspaces } from '$lib/sidebar/thread-list-store.svelte'
import { isShortcut } from '$lib/keyboard/shortcuts'
import type { Place } from './places'

/** Where the app is, as a place the switcher knows, or null (Settings, a scratch, a repo). */
export function currentPlace(): Place | null {
  if (nav.view === 'workspace' && nav.workspaceId)
    return { kind: 'workspace', id: nav.workspaceId }
  if (nav.view === 'newThread' || nav.view === 'sprint')
    return { kind: nav.view }
  return null
}

function openPlace(place: Place) {
  if (place.kind === 'workspace') {
    const thread = switcher.lastThread[place.id]
    const stillThere =
      thread && threads.getById(thread)?.workspaceId === place.id
    openWorkspace(place.id, { threadId: stillThere ? thread : undefined })
  } else if (place.kind === 'newThread') navigate({ view: 'newThread' })
  else navigate({ view: 'sprint' })
}

/** Goes to the selected place in the open switcher, or the one at `index` (a click). */
export function commitPlace(index?: number) {
  const place = switcher.commit(index)
  if (place) openPlace(place)
}

/**
 * Ctrl+Tab opens the switcher, or moves through it while Ctrl is held; ⇧ goes back.
 * While it's open, ↵ goes and Esc cancels. Returns whether the key was the switcher's.
 */
export function handleSwitcherKeydown(event: KeyboardEvent): boolean {
  if (isShortcut(event, 'switcher')) {
    event.preventDefault()
    if (shellDialogs.anyOpen()) return true
    switcher.step(event.shiftKey ? -1 : 1, currentPlace(), {
      workspaceIds: sidebarWorkspaces.ordered.map((workspace) => workspace.id),
      sprintAvailable: clickup.configured,
    })
    return true
  }
  if (!switcher.open) return false
  if (event.key === 'Escape') {
    event.preventDefault()
    switcher.cancel()
    return true
  }
  if (event.key === 'Enter') {
    event.preventDefault()
    commitPlace()
    return true
  }
  return false
}

/** Letting go of Ctrl goes to the selected place, the way the macOS app switcher does. */
export function handleSwitcherKeyup(event: KeyboardEvent) {
  if (event.key === 'Control' && switcher.open) commitPlace()
}
