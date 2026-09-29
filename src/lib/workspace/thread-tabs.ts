import type { Thread } from '$lib/state/threads.svelte'
import { plural, threadActivityLine } from '$lib/sidebar/status'

/** Deferred per product/features/10-thread-tabs.md — COR-123 tracks these. */
export const THREAD_TAB_KNOWN_GAPS = [
  'No rename or reorder for threads, and no reopening a closed one',
  'No ⌘1–⌘9 jump to thread N',
  'No unread indicator when a thread posts while you are elsewhere',
  'New threads always use the default engine; no role picker',
] as const

export type ThreadBarTabKey =
  | { kind: 'thread'; threadId: string }
  | { kind: 'findings' }
  | { kind: 'changes' }
  | { kind: 'output' }

export function threadTabAriaLabel(thread: Thread): string {
  const activity = threadActivityLine(thread)
  const base = `${thread.role}, ${activity}`
  if (thread.pendingApprovals <= 0) return base
  return `${base}, ${plural(thread.pendingApprovals, 'approval', 'approvals')} waiting`
}

export function findingsTabAriaLabel(openCount: number): string {
  return `Review findings, ${plural(openCount, 'open finding', 'open findings')}`
}

export function changesTabAriaLabel(countLabel: string): string {
  return countLabel ? `Changes, ${countLabel}` : 'Changes, none yet'
}

export type OutputTabAppStatus = 'stopped' | 'starting' | 'running' | 'crashed'

export function outputTabState(input: {
  provisioning: boolean
  appStatus: OutputTabAppStatus
  port: number | null
}): string {
  if (input.provisioning) return 'setting up'
  if (input.appStatus === 'starting') return 'starting'
  if (input.appStatus === 'running' && input.port != null) {
    return `running on localhost:${input.port}`
  }
  if (input.appStatus === 'crashed') return 'crashed'
  return 'stopped'
}

export function outputTabAriaLabel(input: {
  provisioning: boolean
  appStatus: OutputTabAppStatus
  port: number | null
}): string {
  return `App output, ${outputTabState(input)}`
}

export function buildThreadBarTabOrder(
  threads: Thread[],
  showFindings: boolean,
): ThreadBarTabKey[] {
  const keys: ThreadBarTabKey[] = threads.map((thread) => ({
    kind: 'thread',
    threadId: thread.id,
  }))
  if (showFindings) keys.push({ kind: 'findings' })
  keys.push({ kind: 'changes' }, { kind: 'output' })
  return keys
}

export function activeThreadBarTabKey(input: {
  tabs: ThreadBarTabKey[]
  panelTab: 'thread' | 'findings' | 'changes' | 'output'
  threadId: string | null
}): ThreadBarTabKey | null {
  if (input.panelTab !== 'thread') {
    const kind = input.panelTab
    return input.tabs.find((tab) => tab.kind === kind) ?? null
  }
  return (
    input.tabs.find(
      (tab) => tab.kind === 'thread' && tab.threadId === input.threadId,
    ) ??
    input.tabs.find((tab) => tab.kind === 'thread') ??
    null
  )
}

export function tabKeyId(tab: ThreadBarTabKey): string {
  if (tab.kind === 'thread') return `thread-tab-${tab.threadId}`
  if (tab.kind === 'findings') return 'thread-tab-findings'
  if (tab.kind === 'changes') return 'thread-tab-changes'
  return 'thread-tab-output'
}

export function moveTabFocusIndex(
  current: number,
  length: number,
  key: 'ArrowLeft' | 'ArrowRight' | 'Home' | 'End',
): number {
  if (length <= 0) return 0
  if (key === 'Home') return 0
  if (key === 'End') return length - 1
  if (key === 'ArrowRight') return (current + 1) % length
  return (current - 1 + length) % length
}

export function showFindingsTab(input: {
  workspaceKind: 'review' | null
  reviewReady: boolean
}): boolean {
  return input.workspaceKind === 'review' && input.reviewReady
}
