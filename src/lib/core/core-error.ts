import type { CoreError } from '$lib/ipc'
import { plural } from '$lib/format/plural'

/** Human-readable text for a core error; GitConflict carries an object, not a string. */
export function coreErrorText(
  error: CoreError,
  fallback = 'Something went wrong',
) {
  if (error.kind === 'GitConflict') {
    const paths = error.message.paths?.length ?? 0
    return `Git stopped with ${plural(paths, 'conflicting file')}`
  }
  return error.message || fallback
}
