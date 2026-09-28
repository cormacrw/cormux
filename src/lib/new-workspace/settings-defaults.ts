import type { EngineKind, SettingRow } from '$lib/ipc/bindings'

const ENGINE_VALUES: EngineKind[] = ['claude', 'cursor', 'codex', 'gemini']

export function readDefaultEngine(rows: SettingRow[]): EngineKind {
  const value = rows.find((row) => row.key === 'defaultEngine')?.value
  if (value && ENGINE_VALUES.includes(value as EngineKind)) {
    return value as EngineKind
  }
  return 'claude'
}

export function readDefaultBase(rows: SettingRow[]): string {
  return rows.find((row) => row.key === 'defaultBase')?.value?.trim() || 'main'
}
