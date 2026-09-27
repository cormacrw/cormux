export type ViewId = 'homebase' | 'workspace' | 'settings'

export function resolveWindowTitle(
  view: ViewId,
  workspaceId: string | null,
  workspaceName: string | undefined,
): string {
  if (view === 'settings') return 'Cormux · Settings'
  if (view === 'workspace' && workspaceId) {
    return `Cormux · ${workspaceName ?? workspaceId}`
  }
  return 'Cormux · Homebase'
}
