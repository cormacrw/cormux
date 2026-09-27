import type { Snapshot, StateChanged } from '$lib/ipc'
import type { WorkspaceLifecycle } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import { app } from './app.svelte'
import { memory } from './memory.svelte'
import { prs } from './prs.svelte'
import { settings } from './settings.svelte'
import { threads } from './threads.svelte'
import {
  workspaces,
  type Workspace,
  type WorkspaceCardStatus,
} from './workspaces.svelte'

export { app } from './app.svelte'
export { memory } from './memory.svelte'
export { prs } from './prs.svelte'
export { settings } from './settings.svelte'
export { threads } from './threads.svelte'
export { workspaces } from './workspaces.svelte'

function mapCardStatus(lifecycle: WorkspaceLifecycle): WorkspaceCardStatus {
  if (
    lifecycle === 'running' ||
    lifecycle === 'provisioning' ||
    lifecycle === 'creating'
  ) {
    return 'working'
  }
  if (lifecycle === 'waiting') return 'needsAttention'
  return 'idle'
}

function workspaceActivityText(lifecycle: WorkspaceLifecycle): string {
  switch (lifecycle) {
    case 'ready':
      return 'Ready'
    case 'waiting':
      return 'Waiting for approval'
    case 'provisioningFailed':
      return 'Setup failed'
    case 'tearingDown':
      return 'Tearing down'
    default:
      return 'Idle'
  }
}

function defaultThreadActivity(status: string): string {
  switch (status) {
    case 'running':
      return 'Working'
    case 'waiting':
      return 'Waiting for approval'
    case 'paused':
      return 'Paused'
    default:
      return 'Idle'
  }
}

function pendingByThread(approvals: Snapshot['persisted']['approvals']) {
  const map = new Map<string, number>()
  for (const row of approvals) {
    if (row.status !== 'pending') continue
    map.set(row.threadId, (map.get(row.threadId) ?? 0) + 1)
  }
  return map
}

function buildWorkspaceModels(snapshot: Snapshot): Workspace[] {
  const pendingByWs = new Map<string, number>()
  const pendingThreads = pendingByThread(snapshot.persisted.approvals)
  for (const thread of snapshot.persisted.threads) {
    const count = pendingThreads.get(thread.id) ?? 0
    if (count > 0) {
      pendingByWs.set(
        thread.workspaceId,
        (pendingByWs.get(thread.workspaceId) ?? 0) + count,
      )
    }
  }

  const threadsByWs = new Map<string, typeof snapshot.persisted.threads>()
  for (const thread of snapshot.persisted.threads) {
    const list = threadsByWs.get(thread.workspaceId) ?? []
    list.push(thread)
    threadsByWs.set(thread.workspaceId, list)
  }

  return snapshot.workspaces.map((workspace) => {
    const wsThreads = threadsByWs.get(workspace.id) ?? []
    const runningLike = wsThreads.filter(
      (t) =>
        t.status === 'running' ||
        t.status === 'paused' ||
        t.status === 'provisioning',
    )
    const paused =
      workspace.status === 'running' &&
      runningLike.length > 0 &&
      runningLike.every((t) => t.status === 'paused')

    return {
      id: workspace.id,
      name: workspace.name,
      lifecycle: workspace.status,
      paused,
      activityText: workspaceActivityText(workspace.status),
      pendingApprovals: pendingByWs.get(workspace.id) ?? 0,
      cardStatus: mapCardStatus(workspace.status),
    }
  })
}

function buildThreadModels(snapshot: Snapshot) {
  const pendingThreads = pendingByThread(snapshot.persisted.approvals)
  const workspaceOrder = snapshot.workspaces.map((w) => w.id)
  const orderIndex = new Map(workspaceOrder.map((id, i) => [id, i]))

  const rows = snapshot.persisted.threads.map((thread) => ({
    id: thread.id,
    workspaceId: thread.workspaceId,
    role: thread.title,
    engine: thread.engine,
    status: thread.status,
    paused: thread.status === 'paused',
    activity: defaultThreadActivity(thread.status),
    pendingApprovals: pendingThreads.get(thread.id) ?? 0,
  }))

  rows.sort((a, b) => {
    const wa = orderIndex.get(a.workspaceId) ?? 0
    const wb = orderIndex.get(b.workspaceId) ?? 0
    if (wa !== wb) return wa - wb
    return a.id.localeCompare(b.id)
  })

  return rows
}

export function hydrateFromSnapshot(snapshot: Snapshot) {
  const threadModels = buildThreadModels(snapshot)
  settings.hydrate({
    reduceMotion: snapshot.persisted.settings.some(
      (row) => row.key === 'reduceMotion' && row.value === 'true',
    ),
    autoApproveReadOnly: snapshot.persisted.settings.every(
      (row) => row.key !== 'autoApproveReadOnly' || row.value === 'true',
    ),
  })
  workspaces.hydrate(buildWorkspaceModels(snapshot))
  threads.hydrate(threadModels)
  app.hydrate(snapshot)
  memory.hydrate(snapshot.memory)
  prs.hydrate(
    snapshot.persisted.pullRequests.map((pr) => ({
      id: pr.id,
      title: pr.title,
      repo: pr.repoId ?? '',
      number: pr.number,
    })),
  )
}

export async function patchFromEvent(event: StateChanged) {
  app.version = event.version
  if (event.kind === 'metrics') {
    const result = await commands.getMetrics()
    if (result.status === 'ok') {
      memory.hydrate(result.data)
    }
  }
}
