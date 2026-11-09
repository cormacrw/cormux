import { toastCoreError } from '$lib/feedback/wire-feedback'
import { stacks } from './stacks.svelte'

export async function loadStack(workspaceId: string) {
  const error = await stacks.load(workspaceId)
  if (error) toastCoreError(error)
}
