import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'

export type EngineOption = {
  kind: EngineKind
  label: string
  hint: string
}

export const ENGINE_OPTIONS: EngineOption[] = [
  {
    kind: 'claude',
    label: 'Claude Code',
    hint: 'Edits files directly and asks before risky steps',
  },
  {
    kind: 'cursor',
    label: 'Cursor CLI',
    hint: 'Writes a plan first and waits for your go-ahead',
  },
  {
    kind: 'codex',
    label: 'Codex CLI',
    hint: 'Works through the task in a sandbox and proposes a diff',
  },
  {
    kind: 'gemini',
    label: 'Gemini CLI',
    hint: 'Reads widely across the repo before suggesting changes',
  },
]

export function engineHint(kind: EngineKind): string {
  return ENGINE_OPTIONS.find((row) => row.kind === kind)?.hint ?? ''
}

export function engineInstallLabel(
  status: EngineStatus | undefined,
): string | null {
  if (!status) return null
  if (!status.installed) return 'Not installed'
  if (status.signedIn === false) return 'Sign in required'
  return null
}
