import type { DiffFile } from '$lib/ipc/bindings'
import { subscribeDiffs } from '$lib/ipc'
import {
  totalsFromDiffFiles,
  type DiffLineTotals,
} from '$lib/workspace/diff-totals'

/** A diff fetch in flight. `base` is set when the target changed, so the old files are stale. */
type PendingDiff = { retarget: false } | { retarget: true; base: string | null }

const emptyTotals = (): DiffLineTotals => ({ added: 0, deleted: 0 })

export class WorkspaceDiffStore {
  filesByWorkspace = $state<Record<string, DiffFile[]>>({})
  /** Branch each diff is taken against; `null` means uncommitted changes. */
  baseByWorkspace = $state<Record<string, string | null>>({})
  // Raw so `fetch` can tell its own entry apart from a newer one by identity.
  pendingByWorkspace = $state.raw<Record<string, PendingDiff>>({})

  totals(workspaceId: string): DiffLineTotals {
    const files = this.filesByWorkspace[workspaceId]
    if (!files?.length) return emptyTotals()
    return totalsFromDiffFiles(files)
  }

  base(workspaceId: string): string | null {
    return this.baseByWorkspace[workspaceId] ?? null
  }

  pending(workspaceId: string): PendingDiff | null {
    return this.pendingByWorkspace[workspaceId] ?? null
  }

  /**
   * Track a diff fetch. The result arrives on the diff channel, so a retarget
   * stays pending until an update for the new base lands; a plain refresh
   * clears when the command returns, since an unchanged diff sends nothing.
   */
  async fetch(
    workspaceId: string,
    run: () => Promise<boolean>,
    retarget?: { base: string | null },
  ) {
    const pending: PendingDiff = retarget
      ? { retarget: true, base: retarget.base }
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

  setFiles(workspaceId: string, files: DiffFile[], base: string | null = null) {
    const pending = this.pendingByWorkspace[workspaceId]
    if (pending && (!pending.retarget || pending.base === base))
      this.clearPending(workspaceId)
    this.filesByWorkspace = {
      ...this.filesByWorkspace,
      [workspaceId]: files,
    }
    this.baseByWorkspace = { ...this.baseByWorkspace, [workspaceId]: base }
  }

  clearWorkspace(workspaceId: string) {
    this.clearPending(workspaceId)
    if (!this.filesByWorkspace[workspaceId]) return
    const next = { ...this.filesByWorkspace }
    delete next[workspaceId]
    this.filesByWorkspace = next
    const bases = { ...this.baseByWorkspace }
    delete bases[workspaceId]
    this.baseByWorkspace = bases
  }
}

export const workspaceDiff = new WorkspaceDiffStore()

export function bindWorkspaceDiffSubscription(workspaceId: string) {
  const stop = subscribeDiffs(workspaceId, (update) => {
    if (update.diff?.files) {
      workspaceDiff.setFiles(workspaceId, update.diff.files, update.diff.base)
    }
  })

  return () => {
    stop()
    workspaceDiff.clearWorkspace(workspaceId)
  }
}
