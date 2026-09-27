/** Tracks modal dialogs that block the command palette (spec §02). */
export class ShellDialogsStore {
  newWorkspaceOpen = $state(false)
  confirmOpen = $state(false)

  blocksCommandPalette() {
    return this.newWorkspaceOpen || this.confirmOpen
  }
}

export const shellDialogs = new ShellDialogsStore()
