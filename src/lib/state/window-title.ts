export type ViewId = 'homebase' | 'workspace' | 'scratch' | 'settings' | 'todos'

export function resolveWindowTitle(
  view: ViewId,
  openId: string | null,
  openName: string | undefined,
  appName = 'Cormux',
): string {
  if (view === 'settings') return `${appName} · Settings`
  if (view === 'todos') return `${appName} · TODOs`
  if ((view === 'workspace' || view === 'scratch') && openId) {
    return `${appName} · ${openName ?? openId}`
  }
  return `${appName} · Homebase`
}
