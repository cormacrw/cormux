import type { FindingRow } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import {
  applyQuickSelect,
  defaultSelectedIds,
  emptySelection,
  mergeSelection,
  setGroupSelection,
  type QuickSelectMode,
  toggleFinding,
} from '$lib/findings/selection'
import { openFindings, type FindingSeverity } from '$lib/findings/types'

export class FindingsStore {
  items = $state<FindingRow[]>([])
  private selection = $state<Record<string, Set<string>>>({})
  private selectionTouched = $state<Record<string, boolean>>({})
  targetThreadId = $state<Record<string, string>>({})

  hydrate(items: FindingRow[]) {
    this.items = items
    const byWorkspace: Record<string, FindingRow[]> = {}
    for (const row of items) {
      const bucket = byWorkspace[row.workspaceId] ?? []
      bucket.push(row)
      byWorkspace[row.workspaceId] = bucket
    }
    for (const [workspaceId, rows] of Object.entries(byWorkspace)) {
      const touched = this.selectionTouched[workspaceId] ?? false
      const merged = mergeSelection(
        rows,
        this.selection[workspaceId] ?? emptySelection(),
        touched,
      )
      this.selection = { ...this.selection, [workspaceId]: merged }
    }
  }

  forWorkspace(workspaceId: string) {
    return this.items.filter((row) => row.workspaceId === workspaceId)
  }

  openCount(workspaceId: string) {
    return openFindings(this.forWorkspace(workspaceId)).length
  }

  hasReviewFindings(workspaceId: string) {
    return this.forWorkspace(workspaceId).length > 0
  }

  selectedIds(workspaceId: string): Set<string> {
    return this.selection[workspaceId] ?? emptySelection()
  }

  ensureTargetThread(workspaceId: string, threadIds: string[]) {
    if (!threadIds.length) return
    const current = this.targetThreadId[workspaceId]
    const fallback = threadIds[0]
    if (fallback && (!current || !threadIds.includes(current))) {
      this.targetThreadId = { ...this.targetThreadId, [workspaceId]: fallback }
    }
  }

  setTargetThread(workspaceId: string, threadId: string) {
    this.targetThreadId = { ...this.targetThreadId, [workspaceId]: threadId }
  }

  toggle(workspaceId: string, findingId: string, checked: boolean) {
    const rows = this.forWorkspace(workspaceId)
    const next = toggleFinding(
      this.selectedIds(workspaceId),
      findingId,
      checked,
    )
    this.selection = { ...this.selection, [workspaceId]: next }
    this.selectionTouched = { ...this.selectionTouched, [workspaceId]: true }
    void rows
  }

  quickSelect(workspaceId: string, mode: QuickSelectMode) {
    const rows = this.forWorkspace(workspaceId)
    const next = applyQuickSelect(rows, this.selectedIds(workspaceId), mode)
    this.selection = { ...this.selection, [workspaceId]: next }
    this.selectionTouched = { ...this.selectionTouched, [workspaceId]: true }
  }

  setGroup(workspaceId: string, severity: FindingSeverity, checked: boolean) {
    const rows = this.forWorkspace(workspaceId)
    const next = setGroupSelection(
      rows,
      this.selectedIds(workspaceId),
      severity,
      checked,
    )
    this.selection = { ...this.selection, [workspaceId]: next }
    this.selectionTouched = { ...this.selectionTouched, [workspaceId]: true }
  }

  async sendSelected(workspaceId: string, threadId: string) {
    const ids = [...this.selectedIds(workspaceId)].filter((id) => {
      const row = this.items.find((item) => item.id === id)
      return row?.status === 'open'
    })
    if (!ids.length) return
    const result = await commands.sendWorkspaceFindings({
      workspaceId,
      threadId,
      findingIds: ids,
    })
    if (result.status === 'error') {
      const message =
        typeof result.error.message === 'string'
          ? result.error.message
          : 'Send failed'
      throw new Error(message)
    }
    this.quickSelect(workspaceId, 'none')
  }

  seedDefaults(workspaceId: string) {
    if (this.selectionTouched[workspaceId]) return
    const rows = this.forWorkspace(workspaceId)
    this.selection = {
      ...this.selection,
      [workspaceId]: defaultSelectedIds(rows),
    }
  }
}

export const findings = new FindingsStore()
