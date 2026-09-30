import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'

export type EngineOption = {
  kind: EngineKind
  label: string
}

export const ENGINE_OPTIONS: EngineOption[] = [
  { kind: 'claude', label: 'Claude Code' },
  { kind: 'cursor', label: 'Cursor CLI' },
]

/** Same for every engine. Permission mode lives in Settings. */
export function engineHint(): string {
  return 'Works in the worktree. Asks before each tool unless Run everything is on.'
}

export function engineInstallLabel(
  status: EngineStatus | undefined,
): string | null {
  if (!status) return null
  if (!status.installed) return 'Not installed'
  if (status.signedIn === false) return 'Sign in required'
  return null
}
