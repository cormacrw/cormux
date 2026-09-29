import type { DiffFile } from '$lib/ipc/bindings'
import { subscribeDiffs } from '$lib/ipc'
import {
  totalsFromDiffFiles,
  type DiffLineTotals,
} from '$lib/workspace/diff-totals'

const emptyTotals = (): DiffLineTotals => ({ added: 0, deleted: 0 })

export class WorkspaceDiffStore {
  filesByWorkspace = $state<Record<string, DiffFile[]>>({})
  /** Branch each diff is taken against; `null` means uncommitted changes. */
  baseByWorkspace = $state<Record<string, string | null>>({})

  totals(workspaceId: string): DiffLineTotals {
    const files = this.filesByWorkspace[workspaceId]
    if (!files?.length) return emptyTotals()
    return totalsFromDiffFiles(files)
  }

  base(workspaceId: string): string | null {
    return this.baseByWorkspace[workspaceId] ?? null
  }

  setFiles(workspaceId: string, files: DiffFile[], base: string | null = null) {
    this.filesByWorkspace = {
      ...this.filesByWorkspace,
      [workspaceId]: files,
    }
    this.baseByWorkspace = { ...this.baseByWorkspace, [workspaceId]: base }
  }

  clearWorkspace(workspaceId: string) {
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
