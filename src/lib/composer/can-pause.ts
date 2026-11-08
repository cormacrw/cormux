import type { ThreadStatus } from '$lib/ipc/bindings'

export function composerShowsPauseControl(status: ThreadStatus): boolean {
  return status === 'running' || status === 'paused'
}

export function composerPauseLabel(status: ThreadStatus): string {
  return status === 'paused' ? 'Resume' : 'Pause'
}
