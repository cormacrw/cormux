import { toastCoreError } from './wire-feedback'

/** Why an action failed (a core error, or something thrown), and what it was trying to do. */
export type ActionFailure = { error: unknown; doing: string }

/** Toasts a failure the way `toastCoreError` words any other; true when there was none. */
export function toastFailure(failure: ActionFailure | null): boolean {
  if (failure) toastCoreError(failure.error, failure.doing)
  return failure === null
}
