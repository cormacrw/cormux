/**
 * Structural equality for the plain data the core sends and the timeline builds: primitives,
 * arrays and plain objects. Several times cheaper than comparing `JSON.stringify` output: it
 * allocates nothing (not even a key list) and stops at the first difference, so a reply whose
 * text grew by a chunk is told apart at once rather than after serialising both copies.
 */
export function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (typeof a !== 'object' || typeof b !== 'object' || !a || !b) {
    // NaN is the one value that isn't equal to itself.
    return a !== a && b !== b
  }
  if (Array.isArray(a)) {
    if (!Array.isArray(b) || a.length !== b.length) return false
    for (let index = 0; index < a.length; index += 1) {
      if (!deepEqual(a[index], b[index])) return false
    }
    return true
  }
  if (Array.isArray(b)) return false
  const left = a as Record<string, unknown>
  const right = b as Record<string, unknown>
  let keys = 0
  for (const key in left) {
    keys += 1
    if (!(key in right)) return false
    const x = left[key]
    const y = right[key]
    if (x !== y && !deepEqual(x, y)) return false
  }
  // The same count of keys on the right, so one it has and the left lacks is noticed.
  for (const key in right) {
    if (key) keys -= 1
  }
  return keys === 0
}
