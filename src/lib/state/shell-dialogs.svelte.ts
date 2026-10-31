/** Tracks modal dialogs that block the command palette (spec §02). */
export class ShellDialogsStore {
  /** The scratch waiting on its delete confirmation. */
  endScratch = $state<{ scratchId: string } | null>(null)
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

  openEndScratch(scratchId: string) {
    this.endScratch = { scratchId }
  }

  closeEndScratch() {
    this.endScratch = null
  }

  blocksCommandPalette() {
    return (
      this.endScratch != null ||
      this.confirmOpen ||
      this.teardownWorkspaceId != null ||
      this.createPrWorkspaceId != null ||
      this.submitReviewWorkspaceId != null
    )
  }
}

export const shellDialogs = new ShellDialogsStore()
