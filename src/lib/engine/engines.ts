import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'

type EngineMark = 'CC'

type EngineOption = {
  kind: EngineKind
  label: string
}

export const ENGINE_OPTIONS: EngineOption[] = [
  { kind: 'claude', label: 'Claude Code' },
]

const ENGINE_MARKS: Record<EngineKind, EngineMark> = {
  claude: 'CC',
}

export function engineMark(kind: EngineKind): EngineMark {
  return ENGINE_MARKS[kind]
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

/** Where Settings › Agents is with asking the core which engines are on this Mac. */
export type EngineDetection =
  | { state: 'checking' }
  | { state: 'failed'; error: string }
  | { state: 'detected'; statuses: EngineStatus[] }

/** The line under an engine: still checking, why the check failed, or what it found. */
export function engineStatusLine(
  detection: EngineDetection,
  kind: EngineKind,
): string {
  if (detection.state === 'checking') return 'Checking…'
  if (detection.state === 'failed') return detection.error
  const status = detection.statuses.find((row) => row.kind === kind)
  if (!status?.installed) return 'Not installed'
  const parts: string[] = []
  if (status.binary) parts.push(status.binary)
  if (status.version) parts.push(`v${status.version}`)
  if (status.signedIn === false) parts.push('Sign in required')
  if (status.signedIn === true) parts.push('Signed in')
  return parts.join(' · ') || 'Installed'
}
