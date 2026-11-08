import type {
  GitConflictState,
  ReviewStatus,
  Snapshot,
  WorkspaceAppStatus,
  WorkspaceKind,
  WorkspaceLifecycle,
  WorkspaceRunCommand,
} from '$lib/ipc/bindings'
import { workspaceActivity } from './activity'

/** What only the running app knows about a workspace: its dev server and git state. */
export type WorkspaceRuntime = {
  appStatus: WorkspaceAppStatus
  port: number | null
  /** The app announced an `https://` URL. */
  https: boolean
  behind: number
  ahead: number
  conflict: GitConflictState | null
  exitCode: number | null
  /** Bumped by Clear, so the log view starts over. */
  logVersion: number
}

export const IDLE_RUNTIME: WorkspaceRuntime = Object.freeze({
  appStatus: 'stopped',
  port: null,
  https: false,
  behind: 0,
  ahead: 0,
  conflict: null,
  exitCode: null,
  logVersion: 0,
})

export type Workspace = {
  id: string
  repoId: string
  repoPath: string
  worktreePath: string
  name: string
  branch: string
  /** The branch it's compared with and pulls from. */
  base: string
  kind: WorkspaceKind
  lifecycle: WorkspaceLifecycle
  /** Running, with every agent in it paused. */
  paused: boolean
  activityText: string
  pendingApprovals: number
  createdAtMs: number | null
  summary: string | null
  prNumber: number | null
  prHtmlUrl: string | null
  /** The setup command that failed and its exit code, once setup failed at one. */
  setupFailure: { command: string; exitCode: number } | null
  /** What Run starts here (the workspace's own command, else its repo's), as the core resolved it. */
  run: Omit<WorkspaceRunCommand, 'workspaceId'> | null
  /** Where a review of this workspace stands; null before one starts. */
  reviewStatus: ReviewStatus | null
  /** The review was posted to GitHub. */
  reviewSubmitted: boolean
  runtime: WorkspaceRuntime
}

function countBy<T>(rows: T[], key: (row: T) => string | null) {
  const counts = new Map<string, number>()
  for (const row of rows) {
    const k = key(row)
    if (k !== null) counts.set(k, (counts.get(k) ?? 0) + 1)
  }
  return counts
}

/**
 * The snapshot's live workspaces, each with its row, review, pending approvals and runtime.
 * A runtime the snapshot doesn't mention stays as `previous` had it.
 */
export function workspacesFromSnapshot(
  snapshot: Snapshot,
  previous: (id: string) => WorkspaceRuntime | undefined = () => undefined,
): Workspace[] {
  const { persisted } = snapshot
  const rows = new Map(persisted.workspaces.map((row) => [row.id, row]))
  const reviews = new Map(
    persisted.reviews.map((row) => [row.workspaceId, row]),
  )
  const git = new Map(
    snapshot.workspaceGit.map((row) => [row.workspaceId, row]),
  )
  const apps = new Map(
    snapshot.workspaceApps.map((row) => [row.workspaceId, row]),
  )
  const runs = new Map(
    snapshot.workspaceRunCommands.map(({ workspaceId, ...run }) => [
      workspaceId,
      run,
    ]),
  )
  const workspaceOf = new Map(
    persisted.threads.map((thread) => [thread.id, thread.workspaceId]),
  )
  const pending = countBy(persisted.approvals, (row) =>
    row.status === 'pending' ? (workspaceOf.get(row.threadId) ?? null) : null,
  )
  const runningLike = new Map<string, { all: number; paused: number }>()
  for (const thread of persisted.threads) {
    if (!['running', 'paused', 'provisioning'].includes(thread.status)) continue
    const tally = runningLike.get(thread.workspaceId) ?? { all: 0, paused: 0 }
    tally.all += 1
    if (thread.status === 'paused') tally.paused += 1
    runningLike.set(thread.workspaceId, tally)
  }

  return snapshot.workspaces.map((record) => {
    const row = rows.get(record.id)
    const review = reviews.get(record.id)
    const tally = runningLike.get(record.id)
    const gitRow = git.get(record.id)
    const app = apps.get(record.id)
    const runtime = { ...(previous(record.id) ?? IDLE_RUNTIME) }
    if (gitRow) {
      runtime.behind = gitRow.behind
      runtime.ahead = gitRow.ahead
      runtime.conflict = gitRow.conflict
    }
    if (app) {
      runtime.appStatus = app.status
      runtime.port = app.port
      runtime.https = app.https
      runtime.exitCode = app.exitCode
    }
    return {
      id: record.id,
      repoId: record.repoId,
      repoPath: record.repoPath,
      worktreePath: record.worktreePath,
      name: record.name,
      branch: record.branch,
      base: record.base,
      kind: record.kind,
      lifecycle: record.status,
      paused:
        record.status === 'running' &&
        !!tally &&
        tally.all > 0 &&
        tally.paused === tally.all,
      activityText: workspaceActivity(record),
      pendingApprovals: pending.get(record.id) ?? 0,
      createdAtMs: row?.createdAtMs ?? null,
      summary: row?.summary ?? null,
      prNumber: row?.prNumber ?? null,
      prHtmlUrl: row?.prHtmlUrl ?? null,
      setupFailure:
        record.activity.kind === 'setupFailed' &&
        record.activity.command != null &&
        record.activity.exitCode != null
          ? {
              command: record.activity.command,
              exitCode: record.activity.exitCode,
            }
          : null,
      run: runs.get(record.id) ?? null,
      reviewStatus: review?.status ?? null,
      reviewSubmitted: review?.submitted ?? false,
      runtime,
    }
  })
}
