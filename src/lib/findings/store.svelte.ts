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
import { threads, type Thread } from '$lib/state/threads.svelte'
import { reuseRows } from '../state/reuse-rows'
import { onSnapshot } from '$lib/core/sync'

const NO_FINDINGS: FindingRow[] = []

/**
 * A workspace's findings, which of them are selected and which thread they go to. Until the
 * user picks, the selection is the open blocking findings and the target is the first live
 * thread; only explicit picks are stored.
 */
export class FindingsStore {
  items = $state.raw<FindingRow[]>([])
  private picked = $state<Record<string, Set<string>>>({})
  private pickedThread = $state<Record<string, string>>({})

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

  hasReviewFindings(workspaceId: string) {
    return this.forWorkspace(workspaceId).length > 0
  }

  selectedIds(workspaceId: string): Set<string> {
    return (
      this.picked[workspaceId] ??
      defaultSelectedIds(this.forWorkspace(workspaceId))
    )
  }

  /** The threads findings can be sent to: the workspace's live ones. */
  targetThreads(workspaceId: string): Thread[] {
    return threads
      .forWorkspace(workspaceId)
      .filter((thread) => !thread.retired)
  }

  targetThread(workspaceId: string): Thread | undefined {
    const candidates = this.targetThreads(workspaceId)
    const picked = this.pickedThread[workspaceId]
    return (
      candidates.find((thread) => thread.id === picked) ?? candidates[0]
    )
  }

  setTargetThread(workspaceId: string, threadId: string) {
    this.pickedThread = { ...this.pickedThread, [workspaceId]: threadId }
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

  /** Sends the selected open findings to the target thread; resolves why the core refused, or null. */
  async sendSelected(workspaceId: string): Promise<CoreError | null> {
    const thread = this.targetThread(workspaceId)
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
