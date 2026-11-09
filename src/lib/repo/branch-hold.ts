import type { BranchHold } from '$lib/ipc/bindings'

/** Why a local branch can't be deleted, in a few words for its badge. */
export function branchHoldLabel(hold: BranchHold): string {
  switch (hold.kind) {
    case 'defaultBranch':
      return 'Default branch'
    case 'checkedOutBy':
      return `Checked out by ${hold.workspaces.join(', ')}`
    case 'repoHead':
      return 'Checked out in the repo'
  }
}
