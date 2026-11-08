import type { Component } from 'svelte'
import type { ShellDialogId } from '$lib/state/shell-dialogs.svelte'

export type Loader = () => Promise<{ default: Component }>

export async function loadView(
  load: Loader,
  report: (error: unknown) => void,
): Promise<Component | undefined> {
  try {
    return (await load()).default
  } catch (error) {
    report(error)
    return undefined
  }
}

/** Each shell dialog's component, mounted the first time it opens. */
export const SHELL_DIALOGS: Record<ShellDialogId, Loader> = {
  teardown: () => import('../workspace/TeardownDialog.svelte'),
  createPr: () => import('../workspace/CreatePrDialog.svelte'),
  submitReview: () => import('../workspace/SubmitReviewDialog.svelte'),
  endScratch: () => import('../scratch/EndScratchDialog.svelte'),
}
