import { keyboardLayers } from '$lib/keyboard/layers.svelte'
import { isShortcut, type MatchedShortcutId } from '$lib/keyboard/shortcuts'

/** The app's shell dialogs (mounted from `lazy-views.ts`), each with what it opens on. */
type ShellDialogArgs = {
  teardown: { workspaceId: string }
  createPr: { workspaceId: string }
  submitReview: { workspaceId: string }
  /** The scratch waiting on its delete confirmation. */
  endScratch: { scratchId: string }
}

export type ShellDialogId = keyof ShellDialogArgs

/**
 * The shell dialogs, opened through here with what they open on. Each is a modal layer from the
 * moment it's asked for (a teardown waits on its preview before it shows).
 */
class ShellDialogsStore {
  private openArgs = $state<{ [K in ShellDialogId]?: ShellDialogArgs[K] }>({})

  constructor() {
    keyboardLayers.hold('modal', () => Object.keys(this.openArgs).length > 0)
  }

  open<K extends ShellDialogId>(id: K, args: ShellDialogArgs[K]) {
    keyboardLayers.modalOpening()
    this.openArgs = { ...this.openArgs, [id]: args }
  }

  close(id: ShellDialogId) {
    if (!(id in this.openArgs)) return
    const next = { ...this.openArgs }
    delete next[id]
    this.openArgs = next
  }

  args<K extends ShellDialogId>(id: K): ShellDialogArgs[K] | null {
    return (this.openArgs[id] as ShellDialogArgs[K] | undefined) ?? null
  }

  isOpen(id: ShellDialogId) {
    return this.openArgs[id] != null
  }

  /** A shell dialog or any other modal is open. */
  anyOpen() {
    return keyboardLayers.modal
  }

  /** A shortcut for a button on screen: pressed, not held down, and no dialog open. */
  allows(event: KeyboardEvent, id: MatchedShortcutId) {
    return isShortcut(event, id) && !event.repeat && !this.anyOpen()
  }
}

export const shellDialogs = new ShellDialogsStore()
