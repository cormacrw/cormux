import type { FindingRow } from '$lib/ipc/bindings'

export class FindingsStore {
  items = $state<FindingRow[]>([])

  hydrate(items: FindingRow[]) {
    this.items = items
  }

  forWorkspace(workspaceId: string) {
    return this.items.filter((row) => row.workspaceId === workspaceId)
  }

  openCount(workspaceId: string) {
    return this.forWorkspace(workspaceId).filter((row) => row.status === 'open')
      .length
  }

  hasReviewFindings(workspaceId: string) {
    return this.forWorkspace(workspaceId).length > 0
  }
}

export const findings = new FindingsStore()
