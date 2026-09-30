import type { EngineKind, SettingRow } from '$lib/ipc/bindings'

const ENGINE_VALUES: EngineKind[] = ['claude', 'cursor']

export const DEFAULT_WORKTREE_ROOT = '~/.harness/worktrees'

export function readDefaultEngine(rows: SettingRow[]): EngineKind {
  const value = rows.find((row) => row.key === 'defaultEngine')?.value
  if (value && ENGINE_VALUES.includes(value as EngineKind)) {
    return value as EngineKind
  }
  return 'claude'
}

export function readBooleanSetting(
  rows: SettingRow[],
  key: string,
  defaultValue: boolean,
): boolean {
  const row = rows.find((entry) => entry.key === key)
  if (!row) return defaultValue
  return row.value === 'true'
}

export function readStringSetting(
  rows: SettingRow[],
  key: string,
  defaultValue: string,
): string {
  const value = rows.find((entry) => entry.key === key)?.value?.trim()
  return value || defaultValue
}

/** The saved default repo if it still exists, else the first repo. */
export function resolveDefaultRepoId(
  repos: { id: string }[],
  preferred: string,
): string {
  if (preferred && repos.some((repo) => repo.id === preferred)) return preferred
  return repos[0]?.id ?? ''
}
