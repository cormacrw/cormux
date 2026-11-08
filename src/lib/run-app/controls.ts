import type {
  WorkspaceAppControlAction,
  WorkspaceAppStatus,
  WorkspaceLifecycle,
} from '$lib/ipc/bindings'
import { isWorkspaceProvisioning } from '$lib/state/activity'

export type AppControls = {
  /** Play and ⌘R: `run` when stopped, a fresh `restart` after a crash; null while the app is up. */
  start: { action: 'run' | 'restart'; enabled: boolean } | null
  /** ⌘.: while the app is starting or running. */
  canStop: boolean
  /** Restart a running app; a starting one has nothing to restart yet. */
  canRestart: boolean
  /** What Run starts, as the core resolved it ('' for nothing). */
  runCommand: string
}

/** What Run, Stop and Restart mean for an app in this state, and which of them apply. */
export function appControlsFor(input: {
  appStatus: WorkspaceAppStatus
  lifecycle: WorkspaceLifecycle | undefined
  runCommand: string
}): AppControls {
  const { appStatus, lifecycle, runCommand } = input
  const setupDone =
    lifecycle != null &&
    !isWorkspaceProvisioning(lifecycle) &&
    lifecycle !== 'provisioningFailed'
  const up = appStatus === 'starting' || appStatus === 'running'
  return {
    start: up
      ? null
      : appStatus === 'crashed'
        ? { action: 'restart', enabled: true }
        : { action: 'run', enabled: setupDone && !!runCommand },
    canStop: up,
    canRestart: appStatus === 'running',
    runCommand,
  }
}

export function appActionAllowed(
  controls: AppControls,
  action: WorkspaceAppControlAction,
): boolean {
  switch (action) {
    case 'run':
      return controls.start?.action === 'run' && controls.start.enabled
    case 'restart':
      return controls.start?.action === 'restart' || controls.canRestart
    case 'stop':
      return controls.canStop
    case 'clear':
      return true
  }
}
