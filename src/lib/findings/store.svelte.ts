// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import { SvelteSet } from 'svelte/reactivity'
import type { FindingRow } from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import {
  applyQuickSelect,
  defaultSelectedIds,
  mergeSelection,
  type QuickSelectMode,
  setGroupSelection,
  toggleFinding,
} from '$lib/findings/selection'
import { openFindings, type FindingSeverity } from '$lib/findings/types'
import { reuseRows } from '../state/reuse-rows'
import { onSnapshot } from '$lib/core/sync'

function sameSet(a: Set<string>, b: Set<string>) {
  if (a.size !== b.size) return false
  for (const id of a) if (!b.has(id)) return false
  return true
}

const NO_FINDINGS: FindingRow[] = []

export class FindingsStore {
  items = $state.raw<FindingRow[]>([])
  private selection = $state<Record<string, Set<string>>>({})
  private selectionTouched = $state<Record<string, boolean>>({})
  targetThreadId = $state<Record<string, string>>({})

  private readonly byWorkspace = $derived.by(() => {
    const map = new Map<string, FindingRow[]>()
    for (const row of this.items) {
      const list = map.get(row.workspaceId)
      if (list) list.push(row)
      else map.set(row.workspaceId, [row])
    }
    return map
  })

  hydrate(items: FindingRow[]) {
    const previous = this.items
    this.items = reuseRows(previous, items)
    if (this.items === previous) return
    let selection: Record<string, Set<string>> | null = null
    for (const [workspaceId, rows] of this.byWorkspace) {
      const touched = this.selectionTouched[workspaceId] ?? false
      const current = this.selection[workspaceId]
      // An untouched selection follows the findings; a touched one is the user's.
      if (touched && current) continue
      const merged = mergeSelection(rows, current ?? new SvelteSet(), touched)
      if (current && sameSet(current, merged)) continue
      selection ??= { ...this.selection }
      selection[workspaceId] = merged
    }
    if (selection) this.selection = selection
  }

  /** The workspace's findings; the same array until the list changes. */
  forWorkspace(workspaceId: string): FindingRow[] {
    return this.byWorkspace.get(workspaceId) ?? NO_FINDINGS
  }

  openCount(workspaceId: string) {
    return openFindings(this.forWorkspace(workspaceId)).length
  }

  hasReviewFindings(workspaceId: string) {
    return this.forWorkspace(workspaceId).length > 0
  }

  selectedIds(workspaceId: string): Set<string> {
    return this.selection[workspaceId] ?? new SvelteSet()
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
    const next = toggleFinding(
      this.selectedIds(workspaceId),
      findingId,
      checked,
    )
    this.selection = { ...this.selection, [workspaceId]: next }
    this.selectionTouched = { ...this.selectionTouched, [workspaceId]: true }
  }

  quickSelect(workspaceId: string, mode: QuickSelectMode) {
    const rows = this.forWorkspace(workspaceId)
    const next = applyQuickSelect(rows, this.selectedIds(workspaceId), mode)
    this.selection = { ...this.selection, [workspaceId]: next }
    this.selectionTouched = { ...this.selectionTouched, [workspaceId]: true }
  }

  setGroup(workspaceId: string, severity: FindingSeverity, checked: boolean) {
    const next = setGroupSelection(
      this.forWorkspace(workspaceId),
      this.selectedIds(workspaceId),
      severity,
      checked,
    )
    this.selection = { ...this.selection, [workspaceId]: next }
    this.selectionTouched = { ...this.selectionTouched, [workspaceId]: true }
  }

  /** Sends the selected open findings to the thread; resolves why the core refused, or null. */
  async sendSelected(
    workspaceId: string,
    threadId: string,
  ): Promise<CoreError | null> {
    const open = new Set(
      openFindings(this.forWorkspace(workspaceId)).map((row) => row.id),
    )
    const ids = [...this.selectedIds(workspaceId)].filter((id) => open.has(id))
    if (!ids.length) return null
    const result = await commands.sendWorkspaceFindings({
      workspaceId,
      threadId,
      findingIds: ids,
    })
    if (result.status === 'error') return result.error
    this.quickSelect(workspaceId, 'none')
    return null
  }

  /** Dismisses findings (or restores them to open) and drops them from the selection. */
  async setDismissed(
    workspaceId: string,
    ids: string[],
    dismissed: boolean,
  ): Promise<CoreError | null> {
    if (!ids.length) return null
    const result = await commands.dismissWorkspaceFindings({
      workspaceId,
      findingIds: ids,
      dismissed,
    })
    if (result.status === 'error') return result.error
    const next = new SvelteSet(this.selectedIds(workspaceId))
    for (const id of ids) next.delete(id)
    this.selection = { ...this.selection, [workspaceId]: next }
    this.selectionTouched = { ...this.selectionTouched, [workspaceId]: true }
    return null
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

onSnapshot((snapshot) => findings.hydrate(snapshot.persisted.findings))
