export class LruCache<K, V> {
  private values = new Map<K, V>()

  constructor(private limit: number) {}

  get(key: K): V | undefined {
    if (!this.values.has(key)) return undefined
    const value = this.values.get(key) as V
    this.values.delete(key)
    this.values.set(key, value)
    return value
  }

  set(key: K, value: V): V {
    this.values.delete(key)
    this.values.set(key, value)
    if (this.values.size > this.limit) {
      this.values.delete(this.values.keys().next().value as K)
    }
    return value
  }
}

/** Remembers the last `limit` results by input text, for pure work repeated on the same strings. */
export function memoByText<T>(
  compute: (text: string) => T,
  limit = 500,
): (text: string) => T {
  const cache = new LruCache<string, { value: T }>(limit)
  return (text) =>
    (cache.get(text) ?? cache.set(text, { value: compute(text) })).value
}
