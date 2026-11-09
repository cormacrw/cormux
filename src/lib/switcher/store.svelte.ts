import { nav } from '$lib/core/nav.svelte'
import {
  placeOf,
  startIndex,
  stepIndex,
  switcherPlaces,
  visit,
  type Place,
} from '$lib/switcher/places'

/** Ctrl+Tab: the places visited, most recent first, and the switcher while it's held open. */
class SwitcherStore {
  // Where the app already is counts as the first visit; `onNavigate` below adds the rest.
  history = $state<Place[]>(
    [placeOf(nav.route)].filter((place) => place !== null),
  )
  open = $state(false)
  /** What the open switcher lists, fixed when it opened so it doesn't shift underfoot. */
  list = $state<Place[]>([])
  index = $state(0)
  /** The thread last shown in each workspace, so coming back lands on it. */
  lastThread: Record<string, string> = {}

  visit(place: Place) {
    this.history = visit(this.history, place)
  }

  rememberThread(workspaceId: string, threadId: string) {
    this.lastThread[workspaceId] = threadId
  }

  /** Opens the switcher on the first press, then moves the selection on each one after. */
  step(
    by: 1 | -1,
    current: Place | null,
    input: { workspaceIds: string[]; sprintAvailable: boolean },
  ) {
    if (!this.open) {
      const list = switcherPlaces(this.history, input)
      if (!list.length) return
      this.list = list
      this.index = by === 1 ? startIndex(list, current) : list.length - 1
      this.open = true
      return
    }
    this.index = stepIndex(this.index, this.list.length, by)
  }

  commit(index = this.index): Place | null {
    if (!this.open) return null
    this.open = false
    return this.list[index] ?? null
  }

  cancel() {
    this.open = false
  }
}

export const switcher = new SwitcherStore()

// Ctrl+Tab goes back through the places visited, most recent first.
nav.onNavigate((to) => {
  const place = placeOf(to)
  if (place) switcher.visit(place)
  if (to.view === 'workspace' && to.threadId)
    switcher.rememberThread(to.workspaceId, to.threadId)
})
