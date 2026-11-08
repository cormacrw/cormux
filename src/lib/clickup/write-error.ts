import type { CoreError } from '$lib/ipc'
import { coreErrorText } from '$lib/core/core-error'

/** A write ClickUp refused, and what was being tried. */
export type ClickupWriteError = { error: CoreError; doing: string }

/** `Could not move the task to Done: <what ClickUp said>`. */
export function writeErrorText(failure: ClickupWriteError) {
  return `${failure.doing}: ${coreErrorText(failure.error)}`
}
