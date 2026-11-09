import type {
  WorkspaceAppControlAction,
  WorkspaceAppStatus,
} from '$lib/ipc/bindings'
import { isAppUp } from './status'

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

/** What Run, Stop and Restart mean for an app in this state; whether Run may start is the core's `runBlock`. */
export function appControlsFor(input: {
  appStatus: WorkspaceAppStatus
  runBlock: string | null
  runCommand: string
}): AppControls {
  const { appStatus, runBlock, runCommand } = input
  return {
    start: isAppUp(appStatus)
      ? null
      : appStatus === 'crashed'
        ? { action: 'restart', enabled: true }
        : { action: 'run', enabled: runBlock == null },
    canStop: isAppUp(appStatus),
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

/**
 * What the Output panel shows under its header: the log, unless it's empty with no app up to
 * print to it, when it says why.
 */
export function outputBody(input: {
  provisioning: boolean
  appStatus: WorkspaceAppStatus
  runCommand: string
  hasLines: boolean
}): 'log' | 'noCommand' | 'idle' {
  if (input.provisioning || input.hasLines || isAppUp(input.appStatus))
    return 'log'
  return input.runCommand ? 'idle' : 'noCommand'
}
