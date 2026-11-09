import type { Route } from '$lib/core/route'

/** Somewhere Ctrl+Tab can take you: a workspace or one of the main pages. */
export type Place =
  { kind: 'workspace'; id: string } | { kind: 'newThread' } | { kind: 'sprint' }

/** The route as a place the switcher knows, or null (Settings, a scratch, a repo). */
export function placeOf(route: Route): Place | null {
  if (route.view === 'workspace')
    return { kind: 'workspace', id: route.workspaceId }
  if (route.view === 'newThread' || route.view === 'sprint')
    return { kind: route.view }
  return null
}

const MAIN_PAGES: Place[] = [{ kind: 'newThread' }, { kind: 'sprint' }]

const HISTORY_LIMIT = 40

export function placeKey(place: Place): string {
  return place.kind === 'workspace' ? `workspace:${place.id}` : place.kind
}

export function visit(history: Place[], place: Place): Place[] {
  const key = placeKey(place)
  return [place, ...history.filter((entry) => placeKey(entry) !== key)].slice(
    0,
    HISTORY_LIMIT,
  )
}

/**
 * What the switcher lists: places visited, most recent first, then the rest (workspaces in
 * sidebar order, then pages) so everything is reachable. Deleted workspaces and a Sprint page
 * that isn't set up are left out.
 */
export function switcherPlaces(
  history: Place[],
  input: { workspaceIds: string[]; sprintAvailable: boolean },
): Place[] {
  const available = (place: Place) =>
    place.kind === 'workspace'
      ? input.workspaceIds.includes(place.id)
      : place.kind !== 'sprint' || input.sprintAvailable
  const all: Place[] = [
    ...input.workspaceIds.map((id) => ({ kind: 'workspace' as const, id })),
    ...MAIN_PAGES,
  ]
  const seen = new Set<string>()
  const list: Place[] = []
  for (const place of [...history, ...all]) {
    const key = placeKey(place)
    if (seen.has(key) || !available(place)) continue
    seen.add(key)
    list.push(place)
  }
  return list
}

export function startIndex(list: Place[], current: Place | null): number {
  if (list.length < 2) return 0
  return current && placeKey(list[0]!) === placeKey(current) ? 1 : 0
}

export function stepIndex(index: number, length: number, by: 1 | -1): number {
  if (!length) return 0
  return (index + by + length) % length
}
