export function composerShowsPauseControl(status: string): boolean {
  return status === 'running' || status === 'paused'
}

export function composerPauseLabel(status: string, paused: boolean): string {
  if (status === 'paused' || paused) return 'Resume'
  return 'Pause'
}
