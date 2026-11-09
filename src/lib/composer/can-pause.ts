import { MID_TURN_THREAD_STATUSES, type ThreadStatus } from '$lib/ipc/bindings'

/** Mid-turn by the core's rule: running, or paused partway, so it can be paused or resumed. */
export function composerShowsPauseControl(status: ThreadStatus): boolean {
  return (MID_TURN_THREAD_STATUSES as readonly ThreadStatus[]).includes(status)
}

export function composerPauseLabel(status: ThreadStatus): string {
  return status === 'paused' ? 'Resume' : 'Pause'
}
