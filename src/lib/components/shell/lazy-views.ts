import type { Component } from 'svelte'
import type { ShellDialogId } from '$lib/state/shell-dialogs.svelte'

type Loader = () => Promise<{ default: Component }>

/** Loads a view's component; a chunk that fails to load goes to `report` and resolves undefined. */
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

// Everything off the first screen, so startup only parses what it paints.
export const loadSettings = () =>
  import('../../settings-page/SettingsView.svelte')
export const loadRepo = () => import('../repo/RepoView.svelte')
export const loadScratch = () => import('../scratch/ScratchView.svelte')
export const loadSprint = () =>
  import('../../clickup/components/SprintView.svelte')

/** Each shell dialog's component, mounted the first time it opens. */
export const SHELL_DIALOGS: Record<ShellDialogId, Loader> = {
  teardown: () => import('../workspace/TeardownDialog.svelte'),
  createPr: () => import('../workspace/CreatePrDialog.svelte'),
  submitReview: () => import('../workspace/SubmitReviewDialog.svelte'),
  endScratch: () => import('../scratch/EndScratchDialog.svelte'),
}

/** Fetches them all once the app has settled, so the first click doesn't land on a blank frame. */
export function preloadLazyViews() {
  for (const load of [
    loadSettings,
    loadRepo,
    loadScratch,
    loadSprint,
    ...Object.values(SHELL_DIALOGS),
  ]) {
    void loadView(load, (error) =>
      console.warn('Could not preload a view', error),
    )
  }
}
