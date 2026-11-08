import type { Snapshot, StateChangeKind, StateChanged } from '$lib/ipc'
import { onSnapshotSupervision } from '$lib/feedback/supervision'
import { pullRequestFromRow } from '$lib/homebase/pr-from-payload'
import { parseTimestampMs } from '$lib/homebase/relative-time'
import type {
  WorkspaceLifecycle,
  WorkspaceRecord,
  WorkspaceRow,
} from '$lib/ipc/bindings'
import { fetchSnapshot } from '$lib/ipc'
import { app } from './app.svelte'
import { coalesce } from './coalesce'
import { clickup } from './clickup.svelte'
import { memory } from './memory.svelte'
import { prs } from './prs.svelte'
import { repos } from './repos.svelte'
import { settings } from './settings.svelte'
import { findings } from './findings.svelte'
import { entities } from './entities.svelte'
import { threadTimeline } from './thread-timeline.svelte'
import { threads, type Thread } from './threads.svelte'
import { scratches, scratchFromRow } from './scratches.svelte'
import { workspaceRecords } from './workspace-records.svelte'
import { NO_USAGE } from '$lib/usage/usage'
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
export { clickup } from './clickup.svelte'
export { memory } from './memory.svelte'
export { prs } from './prs.svelte'
export { repos } from './repos.svelte'
export { settings } from './settings.svelte'
export { appearance } from './appearance.svelte'
export { shellDialogs } from './shell-dialogs.svelte'
export { threads } from './threads.svelte'
export { scratches } from './scratches.svelte'
export { stacks } from './stacks.svelte'
export { workspaceRecords } from './workspace-records.svelte'
export { workspaceUi } from './workspace-ui.svelte'
export { workspaceDiff } from './workspace-diff.svelte'
export { workspaces } from './workspaces.svelte'
export { findings } from './findings.svelte'
export { entities } from './entities.svelte'
export { switcher } from './switcher.svelte'
export { threadTimeline } from './thread-timeline.svelte'

type PersistedThread = Snapshot['persisted']['threads'][number]

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

const LIFECYCLE_VALUES = new Set<WorkspaceLifecycle>([
  'creating',
  'provisioning',
  'ready',
  'running',
  'idle',
  'waiting',
  'tearingDown',
  'gone',
  'provisioningFailed',
])

function parseLifecycle(raw: string): WorkspaceLifecycle {
  return LIFECYCLE_VALUES.has(raw as WorkspaceLifecycle)
    ? (raw as WorkspaceLifecycle)
    : 'ready'
}

function persistedWorkspaceRecord(row: WorkspaceRow): WorkspaceRecord {
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

/**
 * Everything the stores need from one snapshot, read in a single pass: the workspace records,
 * the workspace models built on them, and the thread models built on both.
 */
function modelsFromSnapshot(snapshot: Snapshot) {
  const { persisted } = snapshot
  const pendingThreads = pendingByThread(persisted.approvals)
  const threadsByWs = new Map<string, PersistedThread[]>()
  const pendingByWs = new Map<string, number>()
  for (const thread of persisted.threads) {
    const list = threadsByWs.get(thread.workspaceId)
    if (list) list.push(thread)
    else threadsByWs.set(thread.workspaceId, [thread])
    const count = pendingThreads.get(thread.id) ?? 0
    if (count > 0) {
      pendingByWs.set(
        thread.workspaceId,
        (pendingByWs.get(thread.workspaceId) ?? 0) + count,
      )
    }
  }

  const rows = new Map(persisted.workspaces.map((row) => [row.id, row]))
  const records = workspaceRecordsFromSnapshot(snapshot)
  const recordById = new Map(records.map((record) => [record.id, record]))

  const workspaceModels: Workspace[] = records.map((workspace) => {
    const row = rows.get(workspace.id)
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

    const kind = row?.kind === 'review' ? ('review' as const) : null

    // Waiting says what it's waiting for, not whatever the workspace was last doing.
    const activityText =
      workspace.status === 'waiting'
        ? 'Waiting for approval'
        : workspaceActivityFromRecord(workspace.status, workspace, kind)

    return {
      id: workspace.id,
      name: workspace.name,
      branch: workspace.branch,
      lifecycle: workspace.status,
      paused,
      activityText,
      pendingApprovals: pendingByWs.get(workspace.id) ?? 0,
      cardStatus: mapCardStatus(workspace.status),
      createdAtMs: parseTimestampMs(row?.createdAt),
      summary: row?.summary ?? null,
      summaryAtMs: parseTimestampMs(row?.summaryAt ?? null),
      summarySource: row?.summarySource ?? 'Haiku 4.5',
      kind,
      prNumber: row?.prNumber ?? null,
      prHtmlUrl: row?.prHtmlUrl ?? null,
      modifiedFiles: row?.modifiedFiles ?? 0,
      provStep: workspace.provStep ?? 0,
      setupFailedCommand: workspace.setupFailedCommand ?? null,
      setupFailedExitCode: workspace.setupFailedExitCode ?? null,
    }
  })

  const activityByWs = new Map(
    workspaceModels.map((row) => [row.id, row.activityText]),
  )
  // Threads keep the snapshot's creation order within a workspace: the first tab is the
  // workspace's own thread (ids are random UUIDs, so sorting by id shuffled tabs).
  const orderIndex = new Map(records.map((record, index) => [record.id, index]))
  const workspaceThreads: Thread[] = persisted.threads.map((thread) => {
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
      retired: thread.retired,
      parentThreadId: thread.parentThreadId,
      usage: {
        usedTokens: thread.usedTokens,
        contextSize: thread.contextSize,
        costUsd: thread.costUsd,
      },
    }
  })
  workspaceThreads.sort(
    (a, b) =>
      (orderIndex.get(a.workspaceId) ?? 0) -
      (orderIndex.get(b.workspaceId) ?? 0),
  )

  // A scratch's one thread, shaped like a workspace thread so the conversation and composer reuse it.
  const scratchThreads: Thread[] = persisted.scratches.map((row) => ({
    id: row.threadId,
    workspaceId: row.id,
    role: '',
    engine: row.engine,
    status: row.status,
    paused: row.status === 'paused',
    activity: threadActivityLine(row.status),
    pendingApprovals: pendingThreads.get(row.threadId) ?? 0,
    usage: NO_USAGE,
    retired: true,
    scratchId: row.id,
  }))

  return {
    records,
    workspaces: workspaceModels,
    threads: [...workspaceThreads, ...scratchThreads],
  }
}

export type HydrateOptions = {
  /**
   * Whether threads on screen should re-read the store for steps the core wrote itself (a pause,
   * a pull, a new session), which aren't streamed. On by default; a state event that can't have
   * written any turns it off.
   */
  refreshThreads?: boolean
}

export function hydrateFromSnapshot(
  snapshot: Snapshot,
  options: HydrateOptions = {},
) {
  void onSnapshotSupervision(snapshot)
  const models = modelsFromSnapshot(snapshot)
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
  const aliveIds = new Set(models.workspaces.map((row) => row.id))
  if (app.workspaceId && !aliveIds.has(app.workspaceId)) {
    app.openNewThread()
  }
  scratches.hydrate(snapshot.persisted.scratches.map(scratchFromRow))
  if (app.scratchId && !scratches.getById(app.scratchId)) {
    app.openNewThread()
  }
  workspaces.hydrate(models.workspaces)
  threads.hydrate(models.threads)
  threadTimeline.hydrate({
    threadIds: models.threads.map((thread) => thread.id),
    approvals: snapshot.persisted.approvals,
    settings: snapshot.persisted.settings,
    activity: snapshot.persisted.threadActivity,
    refreshWatched: options.refreshThreads ?? true,
  })
  findings.hydrate(snapshot.persisted.findings)
  entities.hydrate(snapshot.persisted.entities)
  repos.hydrate(snapshot.persisted.repos)
  workspaceRecords.hydrate(models.records)
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
  clickup.hydrate(snapshot.clickupConfigured, snapshot.persisted.settings)
  if (app.view === 'sprint' && !clickup.configured) app.openNewThread()
}

/**
 * State changes that only touch what the snapshot carries, never a thread's events: a fetch,
 * a PR sync, the shell environment, the dev app. The rest may have written a step into a
 * thread (a pause, a pull, a new session), which only a re-read of the store brings on screen.
 */
const KINDS_WITHOUT_THREAD_EVENTS = new Set<StateChangeKind>([
  'behindCounts',
  'prSync',
  'environment',
  'workspaceApp',
])

// Set by events that may have written thread steps, consumed by the next snapshot applied.
let threadRefreshDue = false

/** Fetch and apply a snapshot. Calls that land during one share the next, not one each. */
export const refreshSnapshot = coalesce(async () => {
  const snapshot = await fetchSnapshot()
  const refreshThreads = threadRefreshDue
  threadRefreshDue = false
  hydrateFromSnapshot(snapshot, { refreshThreads })
})

export function patchFromEvent(event: StateChanged) {
  app.version = Math.max(app.version, event.version)
  if (event.kind === 'toast') return
  // A fetch or pull moved `origin`, so the repo rows' counts are stale too.
  if (event.kind === 'behindCounts') void repos.refreshGit()
  if (!KINDS_WITHOUT_THREAD_EVENTS.has(event.kind)) threadRefreshDue = true
  void refreshSnapshot()
}

/** After a missed event: anything may have happened, so threads on screen re-read the store too. */
export function resyncAfterGap() {
  threadRefreshDue = true
  void repos.refreshGit()
  return refreshSnapshot()
}
