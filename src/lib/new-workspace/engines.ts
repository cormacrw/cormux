import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'

export type EngineOption = {
  kind: EngineKind
  label: string
}

export const ENGINE_OPTIONS: EngineOption[] = [
  { kind: 'claude', label: 'Claude Code' },
  { kind: 'cursor', label: 'Cursor CLI' },
]

/** The one installed agent, when there is nothing to choose. */
export function soleInstalledEngine(
  statuses: EngineStatus[],
): EngineKind | null {
  const installed = ENGINE_OPTIONS.filter((option) =>
    statuses.some((row) => row.kind === option.kind && row.installed),
  )
  return installed.length === 1 ? installed[0].kind : null
}

export function engineInstallLabel(
  status: EngineStatus | undefined,
): string | null {
  if (!status) return null
  if (!status.installed) return 'Not installed'
  if (status.signedIn === false) return 'Sign in required'
  return null
}
