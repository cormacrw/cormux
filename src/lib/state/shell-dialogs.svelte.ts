/** Tracks modal dialogs that block the command palette (spec §02). */
export class ShellDialogsStore {
  newWorkspaceOpen = $state(false)
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

  blocksCommandPalette() {
    return (
      this.newWorkspaceOpen ||
      this.confirmOpen ||
      this.teardownWorkspaceId != null ||
      this.createPrWorkspaceId != null ||
      this.submitReviewWorkspaceId != null
    )
  }
}

export const shellDialogs = new ShellDialogsStore()
