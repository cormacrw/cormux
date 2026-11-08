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
  conflict: GitConflictState | null
  exitCode: number | null
}

export const IDLE_RUNTIME: WorkspaceRuntime = Object.freeze({
  appStatus: 'stopped',
  port: null,
  https: false,
  behind: 0,
  conflict: null,
  exitCode: null,
})

export type Workspace = {
  id: string
  repoId: string
  worktreePath: string
  name: string
  branch: string
  /** The branch it's compared with and pulls from. */
  base: string
  kind: WorkspaceKind
  lifecycle: WorkspaceLifecycle
  activityText: string
  pendingApprovals: number
  createdAtMs: number | null
  summary: string | null
  prNumber: number | null
  prHtmlUrl: string | null
  /** The setup command that failed and its exit code, once setup failed at one. */
  setupFailure: { command: string; exitCode: number } | null
  /** Why its branch can't be switched now; null when it can. */
  branchLock: string | null
  /** Why "Review changes" can't start now; null when it can. */
  reviewBlock: string | null
  /** Why Run can't start the app now; null when it can. */
  runBlock: string | null
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

/** The snapshot's live workspaces, each with its row, review, pending approvals and runtime. */
export function workspacesFromSnapshot(snapshot: Snapshot): Workspace[] {
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
  const branchLocks = new Map(
    snapshot.workspaceBranchLocks.map((lock) => [
      lock.workspaceId,
      lock.reason,
    ]),
  )
  const reviewBlocks = new Map(
    snapshot.workspaceReviewBlocks.map((block) => [
      block.workspaceId,
      block.reason,
    ]),
  )
  const runBlocks = new Map(
    snapshot.workspaceRunBlocks.map((block) => [
      block.workspaceId,
      block.reason,
    ]),
  )
  const workspaceOf = new Map(
    persisted.threads.map((thread) => [thread.id, thread.workspaceId]),
  )
  const pending = countBy(persisted.approvals, (row) =>
    row.status === 'pending' ? (workspaceOf.get(row.threadId) ?? null) : null,
  )

  return snapshot.workspaces.map((record) => {
    const row = rows.get(record.id)
    const review = reviews.get(record.id)
    const gitRow = git.get(record.id)
    const app = apps.get(record.id)
    const runtime = { ...IDLE_RUNTIME }
    if (gitRow) {
      runtime.behind = gitRow.behind
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
      worktreePath: record.worktreePath,
      name: record.name,
      branch: record.branch,
      base: record.base,
      kind: record.kind,
      lifecycle: record.status,
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
      branchLock: branchLocks.get(record.id) ?? null,
      reviewBlock: reviewBlocks.get(record.id) ?? null,
      runBlock: runBlocks.get(record.id) ?? null,
      run: runs.get(record.id) ?? null,
      reviewStatus: review?.status ?? null,
      reviewSubmitted: review?.submitted ?? false,
      runtime,
    }
  })
}
