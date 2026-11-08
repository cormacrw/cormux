import { untrack } from 'svelte'
import { dismissOpenPopover } from '$lib/keyboard/popover-registry'
import { isShortcut, type MatchedShortcutId } from '$lib/keyboard/shortcuts'
import { workspaces } from './workspaces.svelte'

/** The app's shell dialogs (mounted from `lazy-views.ts`), each with what it opens on. */
type ShellDialogArgs = {
  teardown: { workspaceId: string }
  createPr: { workspaceId: string }
  submitReview: { workspaceId: string }
  /** The scratch waiting on its delete confirmation. */
  endScratch: { scratchId: string }
}

export type ShellDialogId = keyof ShellDialogArgs

const guards: {
  [K in ShellDialogId]?: (args: ShellDialogArgs[K]) => boolean
} = {
  // A PR comes from a workspace's own branch, once; review workspaces submit a review instead.
  createPr: ({ workspaceId }) => {
    const workspace = workspaces.getById(workspaceId)
    return (
      !!workspace && workspace.prNumber == null && workspace.kind !== 'review'
    )
  },
}

/**
 * The app-wide modal gate. Shell dialogs open through here; dialogs that keep their own open
 * state `hold` it while open. Shortcuts and the command palette wait while any is open.
 */
export class ShellDialogsStore {
  private openArgs = $state<{ [K in ShellDialogId]?: ShellDialogArgs[K] }>({})
  private holds = $state(0)

  /** Opens a dialog unless its guard says no; returns whether it opened. */
  open<K extends ShellDialogId>(id: K, args: ShellDialogArgs[K]): boolean {
    const guard = guards[id] as
      ((args: ShellDialogArgs[K]) => boolean) | undefined
    if (guard && !guard(args)) return false
    dismissOpenPopover()
    this.openArgs = { ...this.openArgs, [id]: args }
    return true
  }

  close(id: ShellDialogId) {
    if (!(id in this.openArgs)) return
    const next = { ...this.openArgs }
    delete next[id]
    this.openArgs = next
  }

  /** What the dialog was opened on, or null while it's closed. */
  args<K extends ShellDialogId>(id: K): ShellDialogArgs[K] | null {
    return (this.openArgs[id] as ShellDialogArgs[K] | undefined) ?? null
  }

  isOpen(id: ShellDialogId) {
    return this.openArgs[id] != null
  }

  anyOpen() {
    return this.holds > 0 || Object.keys(this.openArgs).length > 0
  }

  /** A shortcut for a button on screen: pressed, not held down, and no dialog open. */
  allows(event: KeyboardEvent, id: MatchedShortcutId) {
    return isShortcut(event, id) && !event.repeat && !this.anyOpen()
  }

  /** Holds the gate while `isOpen()` is true. Call while a component initialises. */
  hold(isOpen: () => boolean) {
    $effect(() => {
      if (!isOpen()) return
      untrack(() => (this.holds += 1))
      return () => untrack(() => (this.holds -= 1))
    })
  }
}

export const shellDialogs = new ShellDialogsStore()
