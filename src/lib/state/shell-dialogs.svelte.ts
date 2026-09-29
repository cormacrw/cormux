/** Tracks modal dialogs that block the command palette (spec §02). */
export class ShellDialogsStore {
  newWorkspaceOpen = $state(false)
  newScratchOpen = $state(false)
  /** End (from the scratch page) and Delete (from its card) share one confirm. */
  endScratch = $state<{ scratchId: string; fromHome: boolean } | null>(null)
  confirmOpen = $state(false)
  teardownWorkspaceId = $state<string | null>(null)
  createPrWorkspaceId = $state<string | null>(null)
  submitReviewWorkspaceId = $state<string | null>(null)

  openTeardown(workspaceId: string) {
    this.teardownWorkspaceId = workspaceId
  }

  closeTeardown() {
    this.teardownWorkspaceId = null
  }

  openCreatePr(workspaceId: string) {
    this.createPrWorkspaceId = workspaceId
  }

  closeCreatePr() {
    this.createPrWorkspaceId = null
  }

  openSubmitReview(workspaceId: string) {
    this.submitReviewWorkspaceId = workspaceId
  }

  closeSubmitReview() {
    this.submitReviewWorkspaceId = null
  }

  openEndScratch(scratchId: string, fromHome: boolean) {
    this.endScratch = { scratchId, fromHome }
  }

  closeEndScratch() {
    this.endScratch = null
  }

  blocksCommandPalette() {
    return (
      this.newWorkspaceOpen ||
      this.newScratchOpen ||
      this.endScratch != null ||
      this.confirmOpen ||
      this.teardownWorkspaceId != null ||
      this.createPrWorkspaceId != null ||
      this.submitReviewWorkspaceId != null
    )
  }
}

export const shellDialogs = new ShellDialogsStore()
