export type ViewId = 'homebase' | 'workspace' | 'scratch' | 'settings' | 'todos'

export function resolveWindowTitle(
  view: ViewId,
  openId: string | null,
  openName: string | undefined,
): string {
  if (view === 'settings') return 'Cormux · Settings'
  if (view === 'todos') return 'Cormux · TODOs'
  if ((view === 'workspace' || view === 'scratch') && openId) {
    return `Cormux · ${openName ?? openId}`
  }
  return 'Cormux · Homebase'
}
