import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'

export type EngineMark = 'CC'

export type EngineOption = {
  kind: EngineKind
  label: string
}

export const ENGINE_OPTIONS: EngineOption[] = [
  { kind: 'claude', label: 'Claude Code' },
]

const ENGINE_MARKS: Record<string, EngineMark> = {
  claude: 'CC',
}

/** Takes an engine id (`claude`) or its display name (`Claude Code`). */
export function engineMark(engine: string): EngineMark {
  const kind =
    ENGINE_OPTIONS.find((option) => option.label === engine)?.kind ?? engine
  return ENGINE_MARKS[kind] ?? 'CC'
}

export function engineDisplayName(engine: string): string {
  return (
    ENGINE_OPTIONS.find((option) => option.kind === engine)?.label ?? engine
  )
}

export function engineInstallLabel(
  status: EngineStatus | undefined,
): string | null {
  if (!status) return null
  if (!status.installed) return 'Not installed'
  if (status.signedIn === false) return 'Sign in required'
  return null
}
