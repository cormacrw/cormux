// Sections that were renamed; old deep links still land on them.
const RENAMED: Record<string, string> = { engines: 'agents', macros: 'bangs' }

/** The section a deep link means, or `fallback` for one that no longer exists. */
export function normalizeSettingsSection<T extends string>(
  section: string,
  ids: readonly T[],
  fallback: T,
): T {
  const id = RENAMED[section] ?? section
  return ids.find((known) => known === id) ?? fallback
}
