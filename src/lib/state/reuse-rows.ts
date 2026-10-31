/**
 * The next list, keeping the previous object for every row that didn't change, and the previous
 * list if none did, so a snapshot only invalidates what it actually moved.
 */
export function reuseRows<T extends { id: string }>(
  previous: T[],
  next: T[],
): T[] {
  const before = new Map(previous.map((row) => [row.id, row]))
  let changed = previous.length !== next.length
  const rows = next.map((row, index) => {
    const old = before.get(row.id)
    const same = old && JSON.stringify(old) === JSON.stringify(row)
    if (!same || previous[index] !== old) changed = true
    return same ? old : row
  })
  return changed ? rows : previous
}
