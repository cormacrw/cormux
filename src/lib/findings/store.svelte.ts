// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { FindingRow } from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import {
  applyQuickSelect,
  defaultSelectedIds,
  type QuickSelectMode,
  setGroupSelection,
  toggleFinding,
} from '$lib/findings/selection'
import { openFindings, type FindingSeverity } from '$lib/findings/types'
import { threads } from '$lib/state/threads.svelte'
import type { Workspace } from '$lib/state/workspaces.svelte'
import { showFindingsTab } from '$lib/workspace/thread-tabs'
import { reuseRows } from '../state/reuse-rows'
import { onSnapshot } from '$lib/core/sync'

const NO_FINDINGS: FindingRow[] = []

/**
 * A workspace's findings and which of them are selected. Until the user picks, the selection
 * is the open blocking findings; only explicit picks are stored. They go to the workspace's
 * one live thread.
 */
export class FindingsStore {
  items = $state.raw<FindingRow[]>([])
  private picked = $state<Record<string, Set<string>>>({})

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
    this.items = reuseRows(this.items, items)
  }

  /** The workspace's findings; the same array until the list changes. */
  forWorkspace(workspaceId: string): FindingRow[] {
    return this.byWorkspace.get(workspaceId) ?? NO_FINDINGS
  }

  openCount(workspaceId: string) {
    return openFindings(this.forWorkspace(workspaceId)).length
  }

  /** The one rule for whether the Findings tab is on offer, for its button and the open tab alike. */
  tabAvailable(workspace: Workspace): boolean {
    return showFindingsTab({
      workspaceKind: workspace.kind,
      reviewReady: workspace.reviewStatus === 'ready',
      hasFindings: this.forWorkspace(workspace.id).length > 0,
    })
  }

  selectedIds(workspaceId: string): Set<string> {
    return (
      this.picked[workspaceId] ??
      defaultSelectedIds(this.forWorkspace(workspaceId))
    )
  }

  toggle(workspaceId: string, findingId: string, checked: boolean) {
    this.select(
      workspaceId,
      toggleFinding(this.selectedIds(workspaceId), findingId, checked),
    )
  }

  quickSelect(workspaceId: string, mode: QuickSelectMode) {
    const rows = this.forWorkspace(workspaceId)
    this.select(
      workspaceId,
      applyQuickSelect(rows, this.selectedIds(workspaceId), mode),
    )
  }

  setGroup(workspaceId: string, severity: FindingSeverity, checked: boolean) {
    this.select(
      workspaceId,
      setGroupSelection(
        this.forWorkspace(workspaceId),
        this.selectedIds(workspaceId),
        severity,
        checked,
      ),
    )
  }

  async sendSelected(workspaceId: string): Promise<CoreError | null> {
    const thread = threads.liveFor(workspaceId)
    if (!thread) return null
    const open = new Set(
      openFindings(this.forWorkspace(workspaceId)).map((row) => row.id),
    )
    const ids = [...this.selectedIds(workspaceId)].filter((id) => open.has(id))
    if (!ids.length) return null
    const result = await commands.sendWorkspaceFindings({
      workspaceId,
      threadId: thread.id,
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
    const next = new Set(this.selectedIds(workspaceId))
    for (const id of ids) next.delete(id)
    this.select(workspaceId, next)
    return null
  }

  private select(workspaceId: string, ids: Set<string>) {
    this.picked = { ...this.picked, [workspaceId]: ids }
  }
}

export const findings = new FindingsStore()

onSnapshot((snapshot) => findings.hydrate(snapshot.persisted.findings))
