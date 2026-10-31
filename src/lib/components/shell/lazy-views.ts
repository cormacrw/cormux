// Everything off the first screen, so startup only parses what it paints.
export const loadSettings = () => import('../../../routes/Settings.svelte')
export const loadRepo = () => import('../../../routes/Repo.svelte')
export const loadScratch = () => import('../scratch/ScratchView.svelte')
export const loadSprint = () => import('../sprint/SprintView.svelte')
export const loadTeardownDialog = () =>
  import('../workspace/TeardownDialog.svelte')
export const loadSubmitReviewDialog = () =>
  import('../workspace/SubmitReviewDialog.svelte')
export const loadEndScratchDialog = () =>
  import('../scratch/EndScratchDialog.svelte')

/** Fetches them all once the app has settled, so the first click doesn't land on a blank frame. */
export function preloadLazyViews() {
  for (const load of [
    loadSettings,
    loadRepo,
    loadScratch,
    loadSprint,
    loadTeardownDialog,
    loadSubmitReviewDialog,
    loadEndScratchDialog,
  ]) {
    void load()
  }
}
