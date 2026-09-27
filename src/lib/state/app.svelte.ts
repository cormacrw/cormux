import type { AppView, Snapshot } from '$lib/ipc'
import { workspaces } from './workspaces.svelte'
import { threads } from './threads.svelte'
import { resolveWindowTitle, type ViewId } from './window-title'

export type { ViewId } from './window-title'
export { resolveWindowTitle } from './window-title'

export type FocusTarget = 'homebase' | 'settings' | 'workspace'

export class AppStore {
  version = $state(0)
  view = $state<ViewId>('homebase')
  workspaceId = $state<string | null>(null)
  threadId = $state<string | null>(null)

  /** Incremented when a view should move focus to its main heading. */
  focusGeneration = $state(0)
  focusTarget = $state<FocusTarget | null>(null)

  /** Set while the palette is opening (sidebar / menu hooks). */
  commandPaletteRequested = $state(false)

  /** COR-14 replaces this with the New workspace dialog. */
  newWorkspaceRequested = $state(false)

  readonly windowTitle = $derived.by(() =>
    resolveWindowTitle(
      this.view,
      this.workspaceId,
      this.workspaceId ? workspaces.getById(this.workspaceId)?.name : undefined,
    ),
  )

  private requestFocus(target: FocusTarget) {
    this.focusTarget = target
    this.focusGeneration += 1
  }

  hydrate(snapshot: Snapshot) {
    this.version = snapshot.version
    if (snapshot.view === 'homebase') {
      this.view = 'homebase'
      this.workspaceId = null
      this.threadId = null
      return
    }
    if (snapshot.view === 'settings') {
      this.view = 'settings'
      this.workspaceId = null
      this.threadId = null
      return
    }
    this.view = 'workspace'
    this.workspaceId = snapshot.view.workspace.id
    const first = threads.forWorkspace(snapshot.view.workspace.id)[0]
    this.threadId = first?.id ?? null
  }

  openHomebase() {
    this.view = 'homebase'
    this.workspaceId = null
    this.threadId = null
    this.requestFocus('homebase')
  }

  openSettings() {
    if (this.view === 'settings') return
    this.view = 'settings'
    this.workspaceId = null
    this.threadId = null
    this.requestFocus('settings')
  }

  openWorkspace(workspaceId: string, threadId?: string) {
    this.view = 'workspace'
    this.workspaceId = workspaceId
    const list = threads.forWorkspace(workspaceId)
    this.threadId = threadId ?? list[0]?.id ?? null
    this.requestFocus('workspace')
  }

  requestCommandPalette() {
    this.commandPaletteRequested = true
    window.dispatchEvent(new CustomEvent('cormux:command-palette'))
  }

  requestNewWorkspace() {
    this.newWorkspaceRequested = true
    window.dispatchEvent(new CustomEvent('cormux:new-workspace'))
  }

  applyView(view: AppView) {
    if (view === 'homebase') {
      this.view = 'homebase'
      this.workspaceId = null
      this.threadId = null
      return
    }
    if (view === 'settings') {
      this.view = 'settings'
      this.workspaceId = null
      this.threadId = null
      return
    }
    this.view = 'workspace'
    this.workspaceId = view.workspace.id
  }
}

export const app = new AppStore()
