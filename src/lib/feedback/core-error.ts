import type { CoreError } from '$lib/ipc'

/** Human-readable text for a core error; GitConflict carries an object, not a string. */
export function coreErrorText(
  error: CoreError,
  fallback = 'Something went wrong',
) {
  if (error.kind === 'GitConflict') {
    const paths = error.message.paths?.length ?? 0
    return `Git stopped with ${paths} conflicting file${paths === 1 ? '' : 's'}`
  }
  return error.message || fallback
}
