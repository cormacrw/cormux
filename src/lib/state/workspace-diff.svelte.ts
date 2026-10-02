import type { DiffFile, DiffTarget, LineCounts } from '$lib/ipc/bindings'
import { subscribeDiffs } from '$lib/ipc'
import { sameDiffTarget } from '$lib/stack/stack'
import {
  totalsFromDiffFiles,
  type DiffLineTotals,
} from '$lib/workspace/diff-totals'

/** A diff fetch in flight. `target` is set when it changed, so the old files are stale. */
type PendingDiff =
  { retarget: false } | { retarget: true; target: DiffTarget | null }

const emptyTotals = (): DiffLineTotals => ({ added: 0, deleted: 0 })

export class WorkspaceDiffStore {
  filesByWorkspace = $state<Record<string, DiffFile[]>>({})
  /** What each diff shows; `null` means uncommitted changes. */
  targetByWorkspace = $state<Record<string, DiffTarget | null>>({})
  /** Uncommitted lines, whichever diff is showing. */
  uncommittedByWorkspace = $state<Record<string, LineCounts>>({})
  // Raw so `fetch` can tell its own entry apart from a newer one by identity.
  pendingByWorkspace = $state.raw<Record<string, PendingDiff>>({})

  totals(workspaceId: string): DiffLineTotals {
    const files = this.filesByWorkspace[workspaceId]
    if (!files?.length) return emptyTotals()
    return totalsFromDiffFiles(files)
  }

  uncommitted(workspaceId: string): DiffLineTotals {
    return this.uncommittedByWorkspace[workspaceId] ?? emptyTotals()
  }

  target(workspaceId: string): DiffTarget | null {
    return this.targetByWorkspace[workspaceId] ?? null
  }

  pending(workspaceId: string): PendingDiff | null {
    return this.pendingByWorkspace[workspaceId] ?? null
  }

  /**
   * Track a diff fetch. The result arrives on the diff channel, so a retarget
   * stays pending until an update for the new target lands; a plain refresh
   * clears when the command returns, since an unchanged diff sends nothing.
   */
  async fetch(
    workspaceId: string,
    run: () => Promise<boolean>,
    retarget?: { target: DiffTarget | null },
  ) {
    const pending: PendingDiff = retarget
      ? { retarget: true, target: retarget.target }
      : { retarget: false }
    this.pendingByWorkspace = {
      ...this.pendingByWorkspace,
      [workspaceId]: pending,
    }
    const ok = await run().catch(() => false)
    if (
      (!ok || !retarget) &&
      this.pendingByWorkspace[workspaceId] === pending
    ) {
      this.clearPending(workspaceId)
    }
  }

  private clearPending(workspaceId: string) {
    if (!this.pendingByWorkspace[workspaceId]) return
    const next = { ...this.pendingByWorkspace }
    delete next[workspaceId]
    this.pendingByWorkspace = next
  }

  setFiles(
    workspaceId: string,
    files: DiffFile[],
    target: DiffTarget | null = null,
    uncommitted: LineCounts = totalsFromDiffFiles(files),
  ) {
    const pending = this.pendingByWorkspace[workspaceId]
    if (
      pending &&
      (!pending.retarget || sameDiffTarget(pending.target, target))
    )
      this.clearPending(workspaceId)
    this.filesByWorkspace = {
      ...this.filesByWorkspace,
      [workspaceId]: files,
    }
    this.targetByWorkspace = {
      ...this.targetByWorkspace,
      [workspaceId]: target,
    }
    this.uncommittedByWorkspace = {
      ...this.uncommittedByWorkspace,
      [workspaceId]: uncommitted,
    }
  }

  clearWorkspace(workspaceId: string) {
    this.clearPending(workspaceId)
    if (!this.filesByWorkspace[workspaceId]) return
    const next = { ...this.filesByWorkspace }
    delete next[workspaceId]
    this.filesByWorkspace = next
    const targets = { ...this.targetByWorkspace }
    delete targets[workspaceId]
    this.targetByWorkspace = targets
    const uncommitted = { ...this.uncommittedByWorkspace }
    delete uncommitted[workspaceId]
    this.uncommittedByWorkspace = uncommitted
  }
}

export const workspaceDiff = new WorkspaceDiffStore()

export function bindWorkspaceDiffSubscription(workspaceId: string) {
  const stop = subscribeDiffs(workspaceId, (update) => {
    if (update.diff?.files) {
      workspaceDiff.setFiles(
        workspaceId,
        update.diff.files,
        update.diff.target,
        update.diff.uncommitted,
      )
    }
  })

  return () => {
    stop()
    workspaceDiff.clearWorkspace(workspaceId)
  }
}
