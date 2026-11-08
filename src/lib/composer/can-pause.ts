export function composerShowsPauseControl(status: string): boolean {
  return status === 'running' || status === 'paused'
}

export function composerPauseLabel(status: string): string {
  return status === 'paused' ? 'Resume' : 'Pause'
}
