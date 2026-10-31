/** Remembers the last `limit` results by input text, for pure work repeated on the same strings. */
export function memoByText<T>(
  compute: (text: string) => T,
  limit = 500,
): (text: string) => T {
  const cache = new Map<string, T>()
  return (text) => {
    if (cache.has(text)) {
      const value = cache.get(text) as T
      // Re-inserted so the oldest key is always the least recently used.
      cache.delete(text)
      cache.set(text, value)
      return value
    }
    const value = compute(text)
    cache.set(text, value)
    if (cache.size > limit) cache.delete(cache.keys().next().value as string)
    return value
  }
}
