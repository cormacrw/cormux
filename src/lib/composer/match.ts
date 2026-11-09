/** How many options a composer picker lists. */
const MAX_SHOWN = 8

/**
 * The items `starts` accepts first, then the rest `contains` accepts, at most a picker's worth.
 * Both get the query lowercased.
 */
export function prefixFirst<T>(
  items: T[],
  query: string,
  starts: (item: T, needle: string) => boolean,
  contains: (item: T, needle: string) => boolean,
): T[] {
  const needle = query.toLowerCase()
  const first = items.filter((item) => starts(item, needle))
  const then = items.filter(
    (item) => !first.includes(item) && contains(item, needle),
  )
  return [...first, ...then].slice(0, MAX_SHOWN)
}

export function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
