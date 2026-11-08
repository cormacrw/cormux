import type { DiffFile } from '$lib/ipc/bindings'
import { nav } from '$lib/core/nav.svelte'
import { isDiffCollapsed } from './file-status'

type FileRef = Pick<DiffFile, 'path' | 'added' | 'deleted'>

/** How the Changes panel shows the open workspace's diff: folds and layout. */
class ChangesUiStore {
  /** Files the user folded (true) or unfolded (false); others use the default. */
  private collapsed = $state<Record<string, boolean>>({})
  diffMode = $state<'unified' | 'split'>('unified')

  isCollapsed(file: FileRef) {
    return isDiffCollapsed(this.collapsed, file)
  }

  toggle(file: FileRef) {
    this.setCollapsed([file], !this.isCollapsed(file))
  }

  setCollapsed(files: FileRef[], collapsed: boolean) {
    const next = { ...this.collapsed }
    for (const file of files) next[file.path] = collapsed
    this.collapsed = next
  }

  reset() {
    this.collapsed = {}
  }
}

export const changesUi = new ChangesUiStore()

// Folds belong to one workspace's diff.
nav.onNavigate((to, from) => {
  const leaving =
    from.view === 'workspace' &&
    (to.view !== 'workspace' || to.workspaceId !== from.workspaceId)
  if (leaving) changesUi.reset()
})
