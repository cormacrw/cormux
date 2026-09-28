/** Tracks modal dialogs that block the command palette (spec §02). */
export class ShellDialogsStore {
  newWorkspaceOpen = $state(false)
  confirmOpen = $state(false)
  teardownWorkspaceId = $state<string | null>(null)

  openTeardown(workspaceId: string) {
    this.teardownWorkspaceId = workspaceId
  }

  closeTeardown() {
    this.teardownWorkspaceId = null
  }

  blocksCommandPalette() {
    return (
      this.newWorkspaceOpen ||
      this.confirmOpen ||
      this.teardownWorkspaceId != null
    )
  }
}

export const shellDialogs = new ShellDialogsStore()
