/**
 * UI state and replies that belong to one owner (a thread, a workspace, a draft) must not outlive
 * it or land on whichever owner is current when they arrive. These two helpers are how.
 */

/**
 * Seeds a writable `$derived` that belongs to `owner`: it's written like `$state`, and goes back
 * to `initial` whenever `owner` changes. `let caret = $derived(startsOver(draftId, 0))`.
 */
export function startsOver<T>(owner: unknown, initial: T): T {
  void owner
  return initial
}

/**
 * Numbers each request made for a key. The check it returns is true until a newer request for
 * the same key starts, so only the latest reply lands.
 */
export function latestRequests<K = string>() {
  const latest = new Map<K, number>()
  return (key: K): (() => boolean) => {
    const ticket = (latest.get(key) ?? 0) + 1
    latest.set(key, ticket)
    return () => latest.get(key) === ticket
  }
}
