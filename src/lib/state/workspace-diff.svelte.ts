import type { DiffFile } from '$lib/ipc/bindings'
import { subscribeDiffs } from '$lib/ipc'
import {
  totalsFromDiffFiles,
  type DiffLineTotals,
} from '$lib/workspace/diff-totals'

const emptyTotals = (): DiffLineTotals => ({ added: 0, deleted: 0 })

export class WorkspaceDiffStore {
  filesByWorkspace = $state<Record<string, DiffFile[]>>({})

  totals(workspaceId: string): DiffLineTotals {
    const files = this.filesByWorkspace[workspaceId]
    if (!files?.length) return emptyTotals()
    return totalsFromDiffFiles(files)
  }

  setFiles(workspaceId: string, files: DiffFile[]) {
    this.filesByWorkspace = {
      ...this.filesByWorkspace,
      [workspaceId]: files,
    }
  }

  clearWorkspace(workspaceId: string) {
    if (!this.filesByWorkspace[workspaceId]) return
    const next = { ...this.filesByWorkspace }
    delete next[workspaceId]
    this.filesByWorkspace = next
  }
}

export const workspaceDiff = new WorkspaceDiffStore()

export function bindWorkspaceDiffSubscription(workspaceId: string) {
  const stop = subscribeDiffs(workspaceId, (update) => {
    if (update.diff?.files) {
      workspaceDiff.setFiles(workspaceId, update.diff.files)
    }
  })

  return () => {
    stop()
    workspaceDiff.clearWorkspace(workspaceId)
  }
}
