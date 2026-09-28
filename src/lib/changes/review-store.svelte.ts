export type FileReviewState = 'pending' | 'approved' | 'rejected'

export class ChangesReviewStore {
  byWorkspace = $state<Record<string, Record<string, FileReviewState>>>({})

  review(workspaceId: string, path: string): FileReviewState {
    return this.byWorkspace[workspaceId]?.[path] ?? 'pending'
  }

  setReview(workspaceId: string, path: string, state: FileReviewState) {
    const prev = this.byWorkspace[workspaceId] ?? {}
    this.byWorkspace = {
      ...this.byWorkspace,
      [workspaceId]: { ...prev, [path]: state },
    }
  }

  clearPath(workspaceId: string, path: string) {
    const prev = this.byWorkspace[workspaceId]
    if (!prev?.[path]) return
    const next = { ...prev }
    delete next[path]
    this.byWorkspace = { ...this.byWorkspace, [workspaceId]: next }
  }

  pruneMissing(workspaceId: string, paths: Set<string>) {
    const prev = this.byWorkspace[workspaceId]
    if (!prev) return
    const next: Record<string, FileReviewState> = {}
    for (const [path, state] of Object.entries(prev)) {
      if (paths.has(path)) next[path] = state
    }
    this.byWorkspace = { ...this.byWorkspace, [workspaceId]: next }
  }
}

export const changesReview = new ChangesReviewStore()
