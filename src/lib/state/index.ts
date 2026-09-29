import type { Snapshot, StateChanged } from '$lib/ipc'
import { onSnapshotSupervision } from '$lib/feedback/supervision'
import { pullRequestFromRow } from '$lib/homebase/pr-from-payload'
import { parseTimestampMs } from '$lib/homebase/relative-time'
import type {
  WorkspaceLifecycle,
  WorkspaceRecord,
  WorkspaceRow,
} from '$lib/ipc/bindings'
import { commands, fetchSnapshot } from '$lib/ipc'
import { app } from './app.svelte'
import { homebaseUi } from './homebase-ui.svelte'
import { memory } from './memory.svelte'
import { prs } from './prs.svelte'
import { repos } from './repos.svelte'
import { settings } from './settings.svelte'
import { findings } from './findings.svelte'
import { threadTimeline } from './thread-timeline.svelte'
import { threads, type Thread } from './threads.svelte'
import { scratches, scratchFromRow } from './scratches.svelte'
import { workspaceRecords } from './workspace-records.svelte'
import {
  workspaces,
  type Workspace,
  type WorkspaceCardStatus,
} from './workspaces.svelte'
import {
  threadActivityForProvisioning,
  workspaceActivityFromRecord,
} from '$lib/workspace/provisioning'

export { app } from './app.svelte'
export { memory } from './memory.svelte'
export { prs } from './prs.svelte'
export { repos } from './repos.svelte'
export { settings } from './settings.svelte'
export { appearance } from './appearance.svelte'
export { shellDialogs } from './shell-dialogs.svelte'
export { threads } from './threads.svelte'
export { scratches } from './scratches.svelte'
export { workspaceRecords } from './workspace-records.svelte'
export { homebaseUi } from './homebase-ui.svelte'
export { workspaceUi } from './workspace-ui.svelte'
export { workspaceDiff } from './workspace-diff.svelte'
export { workspaces } from './workspaces.svelte'
export { findings } from './findings.svelte'
export { threadTimeline } from './thread-timeline.svelte'

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

function pendingByThread(approvals: Snapshot['persisted']['approvals']) {
  const map = new Map<string, number>()
  for (const row of approvals) {
    if (row.status !== 'pending') continue
    map.set(row.threadId, (map.get(row.threadId) ?? 0) + 1)
  }
  return map
}

const LIFECYCLE_VALUES: WorkspaceLifecycle[] = [
  'creating',
  'provisioning',
  'ready',
  'running',
  'idle',
  'waiting',
  'tearingDown',
  'gone',
  'provisioningFailed',
]

function parseLifecycle(raw: string): WorkspaceLifecycle {
  if (LIFECYCLE_VALUES.includes(raw as WorkspaceLifecycle)) {
    return raw as WorkspaceLifecycle
  }
  return 'ready'
}

function persistedWorkspaceRecord(row: Snapshot['persisted']['workspaces'][number]): WorkspaceRecord {
  return {
    id: row.id,
    repoId: row.repoId,
    repoPath: '',
    name: row.name,
    branch: row.branch,
    base: 'main',
    worktreePath: row.worktreePath,
    status: parseLifecycle(row.status),
    version: 0,
    activity: '',
    provStep: 0,
    setupFailedCommand: null,
    setupFailedExitCode: null,
  }
}

/** Runtime rows win. Saved workspaces missing from memory still show up so they can be opened. */
function workspaceRecordsFromSnapshot(snapshot: Snapshot): WorkspaceRecord[] {
  const runtimeIds = new Set(snapshot.workspaces.map((row) => row.id))
  const saved = snapshot.persisted.workspaces
    .filter((row) => !row.archivedAt && !runtimeIds.has(row.id))
    .map(persistedWorkspaceRecord)
  return [...snapshot.workspaces, ...saved]
}

function persistedById(snapshot: Snapshot): Map<string, WorkspaceRow> {
  return new Map(snapshot.persisted.workspaces.map((row) => [row.id, row]))
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

  const rows = persistedById(snapshot)
  const records = workspaceRecordsFromSnapshot(snapshot)

  return records.map((workspace) => {
    const persisted = rows.get(workspace.id)
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

    const kind = persisted?.kind === 'review' ? ('review' as const) : null

    const activityText = workspaceActivityFromRecord(
      workspace.status,
      workspace,
      kind,
    )

    return {
      id: workspace.id,
      name: workspace.name,
      branch: workspace.branch,
      lifecycle: workspace.status,
      paused,
      activityText,
      pendingApprovals: pendingByWs.get(workspace.id) ?? 0,
      cardStatus: mapCardStatus(workspace.status),
      createdAtMs: parseTimestampMs(persisted?.createdAt),
      summary: persisted?.summary ?? null,
      summaryAtMs: parseTimestampMs(persisted?.summaryAt ?? null),
      summarySource: persisted?.summarySource ?? 'Haiku 4.5',
      kind,
      prNumber: persisted?.prNumber ?? null,
      prHtmlUrl: persisted?.prHtmlUrl ?? null,
      modifiedFiles: persisted?.modifiedFiles ?? 0,
      provStep: workspace.provStep ?? 0,
      setupFailedCommand: workspace.setupFailedCommand ?? null,
      setupFailedExitCode: workspace.setupFailedExitCode ?? null,
    }
  })
}

function threadActivityLine(status: string): string {
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

function buildThreadModels(snapshot: Snapshot) {
  const pendingThreads = pendingByThread(snapshot.persisted.approvals)
  const workspaceOrder = workspaceRecordsFromSnapshot(snapshot).map((w) => w.id)
  const orderIndex = new Map(workspaceOrder.map((id, i) => [id, i]))

  const records = workspaceRecordsFromSnapshot(snapshot)
  const recordById = new Map(records.map((record) => [record.id, record]))
  const workspaceModels = buildWorkspaceModels(snapshot)
  const activityByWs = new Map(
    workspaceModels.map((row) => [row.id, row.activityText]),
  )

  const rows = snapshot.persisted.threads.map((thread) => {
    const record = recordById.get(thread.workspaceId)
    const provStep = record?.provStep ?? 0
    const wsActivity = activityByWs.get(thread.workspaceId) ?? 'Idle'
    return {
      id: thread.id,
      workspaceId: thread.workspaceId,
      role: thread.title,
      engine: thread.engine,
      status: thread.status,
      paused: thread.status === 'paused',
      activity:
        thread.status === 'provisioning'
          ? threadActivityForProvisioning(thread.status, wsActivity, provStep)
          : threadActivityLine(thread.status),
      pendingApprovals: pendingThreads.get(thread.id) ?? 0,
    }
  })

  rows.sort((a, b) => {
    const wa = orderIndex.get(a.workspaceId) ?? 0
    const wb = orderIndex.get(b.workspaceId) ?? 0
    // Within a workspace keep the snapshot's creation order: the first tab is the
    // workspace's own thread (ids are random UUIDs, so sorting by id shuffled tabs).
    return wa - wb
  })

  return rows
}

/** A scratch's one thread, shaped like a workspace thread so the conversation and composer reuse it. */
function buildScratchThreads(snapshot: Snapshot): Thread[] {
  const pendingThreads = pendingByThread(snapshot.persisted.approvals)
  return snapshot.persisted.scratches.map((row) => ({
    id: row.threadId,
    workspaceId: row.id,
    role: '',
    engine: row.engine,
    status: row.status,
    paused: row.status === 'paused',
    activity: threadActivityLine(row.status),
    pendingApprovals: pendingThreads.get(row.threadId) ?? 0,
    scratchId: row.id,
  }))
}

export function hydrateFromSnapshot(snapshot: Snapshot) {
  void onSnapshotSupervision(snapshot)
  const threadModels = [
    ...buildThreadModels(snapshot),
    ...buildScratchThreads(snapshot),
  ]
  settings.hydrate(
    {
      reduceMotion: snapshot.persisted.settings.some(
        (row) => row.key === 'reduceMotion' && row.value === 'true',
      ),
      autoApproveReadOnly: snapshot.persisted.settings.every(
        (row) => row.key !== 'autoApproveReadOnly' || row.value === 'true',
      ),
    },
    snapshot.persisted.settings,
  )
  const prevIds = new Set(workspaces.items.map((item) => item.id))
  const recordList = workspaceRecordsFromSnapshot(snapshot)
  const nextWorkspaces = buildWorkspaceModels(snapshot)
  if (nextWorkspaces.some((item) => !prevIds.has(item.id))) {
    homebaseUi.resetFilter()
  }
  for (const item of workspaces.items) {
    if (!nextWorkspaces.some((row) => row.id === item.id)) {
      homebaseUi.beginCardExit(item)
    }
  }
  const aliveIds = new Set(nextWorkspaces.map((row) => row.id))
  if (app.workspaceId && !aliveIds.has(app.workspaceId)) {
    app.openHomebase()
  }
  scratches.hydrate(snapshot.persisted.scratches.map(scratchFromRow))
  if (app.scratchId && !scratches.getById(app.scratchId)) {
    app.openHomebase()
  }
  workspaces.hydrate(nextWorkspaces)
  threads.hydrate(threadModels)
  const findingCountByWorkspace: Record<string, number> = {}
  for (const row of snapshot.persisted.findings) {
    findingCountByWorkspace[row.workspaceId] =
      (findingCountByWorkspace[row.workspaceId] ?? 0) + 1
  }
  const threadMeta: Record<
    string,
    {
      status: string
      paused: boolean
      role: string
      workspaceId: string
    }
  > = {}
  for (const thread of threadModels) {
    threadMeta[thread.id] = {
      status: thread.status,
      paused: thread.paused,
      role: thread.role,
      workspaceId: thread.workspaceId,
    }
  }
  threadTimeline.hydrate({
    timeline: snapshot.persisted.timeline,
    threadIds: threadModels.map((thread) => thread.id),
    approvals: snapshot.persisted.approvals,
    settings: snapshot.persisted.settings,
    threadMeta,
    findingCountByWorkspace,
  })
  findings.hydrate(snapshot.persisted.findings)
  repos.hydrate(snapshot.persisted.repos)
  workspaceRecords.hydrate(recordList)
  workspaceRecords.applyGitStats(snapshot.workspaceGit ?? [])
  workspaceRecords.applyApps(snapshot.workspaceApps ?? [])
  app.hydrate(snapshot)
  memory.hydrate(snapshot.memory)
  const prSyncedAtMs = snapshot.prSyncedAt
    ? Number(snapshot.prSyncedAt) * 1000
    : null
  prs.hydrate({
    items: snapshot.persisted.pullRequests
      .map(pullRequestFromRow)
      .filter((row): row is NonNullable<typeof row> => row != null),
    syncedAtMs: Number.isFinite(prSyncedAtMs) ? prSyncedAtMs : null,
    authConfigured: snapshot.githubAuthConfigured,
  })
}

export async function patchFromEvent(event: StateChanged) {
  app.version = event.version
  if (event.kind === 'metrics') {
    const result = await commands.getMetrics()
    if (result.status === 'ok') {
      memory.hydrate(result.data)
    }
    return
  }
  if (event.kind === 'toast') {
    return
  }
  if (event.kind === 'workspaceApp') {
    const snapshot = await fetchSnapshot()
    workspaceRecords.applyApps(snapshot.workspaceApps ?? [])
    app.version = snapshot.version
    return
  }
  if (
    event.kind === 'behindCounts' ||
    event.kind === 'workspaceStatus' ||
    event.kind === 'approvalCounts' ||
    event.kind === 'prSync' ||
    event.kind === 'environment'
  ) {
    hydrateFromSnapshot(await fetchSnapshot())
  }
}
