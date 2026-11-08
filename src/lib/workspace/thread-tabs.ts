import type { WorkspaceAppStatus } from '$lib/ipc/bindings'
import { plural } from '$lib/format/plural'

export function findingsTabAriaLabel(openCount: number): string {
  return `Review findings, ${plural(openCount, 'open finding', 'open findings')}`
}

export function changesTabAriaLabel(countLabel: string | null): string {
  return countLabel ? `Git, ${countLabel}` : 'Git, no changes yet'
}

export function outputTabState(input: {
  provisioning: boolean
  appStatus: WorkspaceAppStatus
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
  appStatus: WorkspaceAppStatus
  port: number | null
}): string {
  return `App output, ${outputTabState(input)}`
}

/** Review workspaces show it once the review is in; others once a branch review found something. */
export function showFindingsTab(input: {
  workspaceKind: 'review' | null
  reviewReady: boolean
  hasFindings: boolean
}): boolean {
  if (input.workspaceKind === 'review') return input.reviewReady
  return input.hasFindings
}
