import { deepEqual } from '$lib/deep-equal'

/**
 * The next list, keeping the previous object for every row that didn't change, and the previous
 * list if none did, so a snapshot only invalidates what it actually moved. Rows are matched by
 * `id` unless `keyOf` says otherwise.
 */
export function reuseRows<T extends object>(
  previous: T[],
  next: T[],
  keyOf: (row: T) => string = (row) => (row as { id: string }).id,
): T[] {
  const before = new Map(previous.map((row) => [keyOf(row), row]))
  let changed = previous.length !== next.length
  const rows = next.map((row, index) => {
    const old = before.get(keyOf(row))
    const same = old !== undefined && deepEqual(old, row)
    if (!same || previous[index] !== old) changed = true
    return same ? old : row
  })
  return changed ? rows : previous
}
