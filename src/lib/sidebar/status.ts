import type { WorkspaceLifecycle } from '$lib/ipc/bindings'

export type StatusDotVariant = 'provisioning' | 'running' | 'paused' | 'idle'

export type WorkspaceSidebarInput = {
  lifecycle: WorkspaceLifecycle
  paused: boolean
  activityText: string
}

export type ThreadSidebarInput = {
  status: string
  paused: boolean
  activity: string
}

export function statusDotVariantForWorkspace(
  workspace: WorkspaceSidebarInput,
): StatusDotVariant {
  if (
    workspace.lifecycle === 'creating' ||
    workspace.lifecycle === 'provisioning'
  ) {
    return 'provisioning'
  }
  if (workspace.lifecycle === 'running') {
    return workspace.paused ? 'paused' : 'running'
  }
  return 'idle'
}

export function statusDotVariantForThread(
  thread: ThreadSidebarInput,
): StatusDotVariant {
  if (thread.status === 'provisioning') return 'provisioning'
  if (thread.status === 'paused' || thread.paused) return 'paused'
  if (thread.status === 'running') return 'running'
  return 'idle'
}

export function workspaceStatusWord(workspace: WorkspaceSidebarInput): string {
  if (
    workspace.lifecycle === 'creating' ||
    workspace.lifecycle === 'provisioning'
  ) {
    return 'Starting'
  }
  if (workspace.lifecycle === 'running') {
    return workspace.paused ? 'Paused' : 'Running'
  }
  return workspace.activityText
}

export function threadActivityLine(thread: ThreadSidebarInput): string {
  if (thread.status === 'paused' || thread.paused) return 'Paused'
  return thread.activity
}

export function isActiveThread(thread: ThreadSidebarInput): boolean {
  return thread.status === 'running' || thread.status === 'provisioning'
}

export function plural(
  count: number,
  singular: string,
  pluralWord = `${singular}s`,
): string {
  return count === 1 ? `1 ${singular}` : `${count} ${pluralWord}`
}

export function formatMemoryGb(bytes: number): string {
  const gb = bytes / 1024 ** 3
  if (gb >= 10) return `${Math.round(gb)} GB`
  return `${gb.toFixed(1)} GB`
}
