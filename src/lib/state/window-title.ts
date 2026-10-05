export type ViewId =
  | 'homebase'
  | 'workspace'
  | 'scratch'
  | 'settings'
  | 'todos'
  | 'sprint'
  | 'repo'

export function resolveWindowTitle(
  view: ViewId,
  openId: string | null,
  openName: string | undefined,
  appName = 'Cormux',
): string {
  if (view === 'settings') return `${appName} · Settings`
  if (view === 'todos') return `${appName} · TODOs`
  if (view === 'sprint') return `${appName} · Sprint`
  if (
    (view === 'workspace' || view === 'scratch' || view === 'repo') &&
    openId
  ) {
    return `${appName} · ${openName ?? openId}`
  }
  return `${appName} · Homebase`
}
