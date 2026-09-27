import type { Snapshot, StateChanged } from '$lib/ipc'
import { app } from './app.svelte'
import { prs } from './prs.svelte'
import { settings } from './settings.svelte'
import { threads } from './threads.svelte'
import { workspaces, type WorkspaceStatus } from './workspaces.svelte'

export { app } from './app.svelte'
export { prs } from './prs.svelte'
export { settings } from './settings.svelte'
export { threads } from './threads.svelte'
export { workspaces } from './workspaces.svelte'

function mapWorkspaceStatus(status: string): WorkspaceStatus {
  if (
    status === 'running' ||
    status === 'provisioning' ||
    status === 'creating'
  ) {
    return 'working'
  }
  if (status === 'waiting') return 'needsAttention'
  return 'idle'
}

export function hydrateFromSnapshot(snapshot: Snapshot) {
  app.hydrate(snapshot)
  settings.hydrate({
    reduceMotion: snapshot.persisted.settings.some(
      (row) => row.key === 'reduceMotion' && row.value === 'true',
    ),
    autoApproveReadOnly: snapshot.persisted.settings.every(
      (row) => row.key !== 'autoApproveReadOnly' || row.value === 'true',
    ),
  })
  workspaces.hydrate(
    snapshot.workspaces.map((workspace) => ({
      id: workspace.id,
      name: workspace.name,
      status: mapWorkspaceStatus(workspace.status),
    })),
  )
  threads.hydrate(
    snapshot.persisted.threads.map((thread) => ({
      id: thread.id,
      workspaceId: thread.workspaceId,
      title: thread.title,
      status: thread.status as 'idle' | 'running' | 'waiting' | 'paused',
    })),
  )
  prs.hydrate(
    snapshot.persisted.pullRequests.map((pr) => ({
      id: pr.id,
      title: pr.title,
      repo: pr.repoId ?? '',
      number: pr.number,
    })),
  )
}

export function patchFromEvent(event: StateChanged) {
  app.version = event.version
}

