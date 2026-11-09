import type { AppStatusView } from '$lib/run-app/status'
import { plural } from '$lib/format/plural'

export function findingsTabAriaLabel(openCount: number): string {
  return `Review findings, ${plural(openCount, 'open finding', 'open findings')}`
}

export function changesTabAriaLabel(countLabel: string | null): string {
  return countLabel ? `Git, ${countLabel}` : 'Git, no changes yet'
}

/** The Output tab's name for screen readers, from the app's one status view. */
export function outputTabAriaLabel(view: AppStatusView): string {
  const state =
    view.phase === 'settingUp'
      ? 'setting up'
      : view.port != null
        ? `running on localhost:${view.port}`
        : view.phase
  return `App output, ${state}`
}
