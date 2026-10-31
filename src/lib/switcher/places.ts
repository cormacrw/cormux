/** Somewhere Ctrl+Tab can take you: a workspace or one of the main pages. */
export type Place =
  { kind: 'workspace'; id: string } | { kind: 'newThread' } | { kind: 'sprint' }

export const MAIN_PAGES: Place[] = [{ kind: 'newThread' }, { kind: 'sprint' }]

const HISTORY_LIMIT = 40

export function placeKey(place: Place): string {
  return place.kind === 'workspace' ? `workspace:${place.id}` : place.kind
}

/** The history with `place` moved to the front: the most recent visit comes first. */
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

/** Where the selection starts: the previous place when you're on the newest one, else the newest. */
export function startIndex(list: Place[], current: Place | null): number {
  if (list.length < 2) return 0
  return current && placeKey(list[0]!) === placeKey(current) ? 1 : 0
}

/** The next selection, wrapping around either end. */
export function stepIndex(index: number, length: number, by: 1 | -1): number {
  if (!length) return 0
  return (index + by + length) % length
}
