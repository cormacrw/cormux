import { app } from '$lib/state/app.svelte'
import { clickup } from '$lib/state/clickup.svelte'
import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
import { switcher } from '$lib/state/switcher.svelte'
import { threads } from '$lib/state/threads.svelte'
import { workspaces } from '$lib/state/workspaces.svelte'
import type { Place } from './places'

/** Where the app is, as a place the switcher knows, or null (Settings, a scratch, a repo). */
export function currentPlace(): Place | null {
  if (app.view === 'workspace' && app.workspaceId)
    return { kind: 'workspace', id: app.workspaceId }
  if (
    app.view === 'homebase' ||
    app.view === 'todos' ||
    app.view === 'week' ||
    app.view === 'sprint'
  )
    return { kind: app.view }
  return null
}

export function openPlace(place: Place) {
  if (place.kind === 'workspace') {
    const thread = switcher.lastThread[place.id]
    const stillThere =
      thread && threads.getById(thread)?.workspaceId === place.id
    app.openWorkspace(place.id, stillThere ? thread : undefined)
  } else if (place.kind === 'homebase') app.openHomebase()
  else if (place.kind === 'todos') app.openTodos()
  else if (place.kind === 'week') app.openWeek()
  else app.openSprint()
}

function commit(index?: number) {
  const place = switcher.commit(index)
  if (place) openPlace(place)
}

/** Picks a place from the open switcher by clicking it. */
export function commitPlace(index: number) {
  commit(index)
}

/**
 * Ctrl+Tab opens the switcher, or moves through it while Ctrl is held; ⇧ goes back.
 * While it's open, ↵ goes and Esc cancels. Returns whether the key was the switcher's.
 */
export function handleSwitcherKeydown(event: KeyboardEvent): boolean {
  if (event.key === 'Tab' && event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault()
    if (shellDialogs.blocksCommandPalette()) return true
    switcher.step(event.shiftKey ? -1 : 1, currentPlace(), {
      workspaceIds: workspaces.sidebarItems.map((workspace) => workspace.id),
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
    commit()
    return true
  }
  return false
}

/** Letting go of Ctrl goes to the selected place, the way the macOS app switcher does. */
export function handleSwitcherKeyup(event: KeyboardEvent) {
  if (event.key === 'Control' && switcher.open) commit()
}
