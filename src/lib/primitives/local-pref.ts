// Per-machine preferences in localStorage. Storage is optional (it can be unavailable, or
// full): a failed read is no preference, and a failed write keeps the choice for this session.

export function readPref(key: string): string | null {
  try {
    return globalThis.localStorage?.getItem(key) ?? null
  } catch {
    return null
  }
}

/** Saves `value` under `key`; null forgets it. */
export function writePref(key: string, value: string | null) {
  try {
    if (value == null) globalThis.localStorage?.removeItem(key)
    else globalThis.localStorage?.setItem(key, value)
  } catch {
    // See above: the choice still holds until the app restarts.
  }
}
