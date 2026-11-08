import type { WorkspaceAppStatus } from '$lib/ipc/bindings'

/** An app starting or running: Stop applies, and its log is live. */
export function isAppUp(status: WorkspaceAppStatus): boolean {
  return status === 'starting' || status === 'running'
}

export type AppPhase =
  'settingUp' | 'starting' | 'running' | 'crashed' | 'stopped'

/** How every place that shows a workspace's app shows it: the Output chip and tab, and the sidebar. */
export type AppStatusView = {
  phase: AppPhase
  /** The chip's word, with a crashed app's exit code. */
  word: string
  /** Setup or the app's start is under way. */
  busy: boolean
  /** Where it serves, once running. */
  port: number | null
}

export function appStatusView(
  runtime: {
    appStatus: WorkspaceAppStatus
    port: number | null
    exitCode: number | null
  },
  provisioning: boolean,
): AppStatusView {
  const phase: AppPhase = provisioning ? 'settingUp' : runtime.appStatus
  const crashed = `Crashed${runtime.exitCode != null ? ` (${runtime.exitCode})` : ''}`
  const words: Record<AppPhase, string> = {
    settingUp: 'Setting up',
    starting: 'Starting',
    running: 'Running',
    crashed,
    stopped: 'Stopped',
  }
  return {
    phase,
    word: words[phase],
    busy: phase === 'settingUp' || phase === 'starting',
    port: runtime.appStatus === 'running' ? runtime.port : null,
  }
}
