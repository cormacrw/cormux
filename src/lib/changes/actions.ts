import { toastCoreError } from '$lib/feedback/wire-feedback'
import { stacks } from './stacks.svelte'

/** Re-reads the workspace's stack, toasting if gh can't tell. */
export async function loadStack(workspaceId: string) {
  const error = await stacks.load(workspaceId)
  if (error) toastCoreError(error)
}
