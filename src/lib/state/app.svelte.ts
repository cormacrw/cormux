import type { AppView, Snapshot } from '$lib/ipc'
import { homebaseUi } from './homebase-ui.svelte'
import { workspaceUi } from './workspace-ui.svelte'
import { workspaces } from './workspaces.svelte'
import { threads } from './threads.svelte'
import { scratches } from './scratches.svelte'
import { clickup } from './clickup.svelte'
import { appName } from '$lib/build-mode'
import { resolveWindowTitle, type ViewId } from './window-title'

export type { ViewId } from './window-title'
export { resolveWindowTitle } from './window-title'

export type FocusTarget =
  'homebase' | 'settings' | 'workspace' | 'scratch' | 'todos' | 'sprint'

export class AppStore {
  version = $state(0)
  view = $state<ViewId>('homebase')
  workspaceId = $state<string | null>(null)
  threadId = $state<string | null>(null)
  /** The open scratch, or null on every other view. */
  scratchId = $state<string | null>(null)
  /** A scratch started without a prompt focuses its composer instead of its title. */
  scratchComposerFocus = $state(false)

  /** Incremented when a view should move focus to its main heading. */
  focusGeneration = $state(0)
  focusTarget = $state<FocusTarget | null>(null)

  /** Set while the palette is opening (sidebar / menu hooks). */
  commandPaletteRequested = $state(false)

  /** COR-14 replaces this with the New workspace dialog. */
  newWorkspaceRequested = $state(false)

  newWorkspacePullRequest = $state<{
    repoId: string | null
    repoFullName: string
    branch: string
    prNumber: number
    title: string
    mode: 'review' | 'continue'
  } | null>(null)

  readonly windowTitle = $derived.by(() => {
    if (this.view === 'scratch') {
      return resolveWindowTitle(
        this.view,
        this.scratchId,
        this.scratchId ? scratches.getById(this.scratchId)?.title : undefined,
        appName,
      )
    }
    return resolveWindowTitle(
      this.view,
      this.workspaceId,
      this.workspaceId ? workspaces.getById(this.workspaceId)?.name : undefined,
      appName,
    )
  })

  private requestFocus(target: FocusTarget) {
    this.focusTarget = target
    this.focusGeneration += 1
  }

  hydrate(snapshot: Snapshot) {
    // ponytail: core always sends view=homebase. Applying it on every snapshot
    // (including the focus refresh a file dialog triggers) leaves Settings.
    // Apply snapshot.view once the core actually stores the open view.
    this.version = snapshot.version
  }

  openHomebase() {
    this.view = 'homebase'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    this.requestFocus('homebase')
  }

  openTodos() {
    this.view = 'todos'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    this.requestFocus('todos')
  }

  /** The ClickUp sprint board, optionally with one task open in the side pane. */
  openSprint(taskId?: string) {
    this.view = 'sprint'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    if (taskId) clickup.select(taskId)
    this.requestFocus('sprint')
  }

  openSettings() {
    if (this.view === 'settings') return
    this.view = 'settings'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    this.requestFocus('settings')
  }

  openWorkspace(workspaceId: string, threadId?: string) {
    const workspace = workspaces.getById(workspaceId)
    if (workspace?.kind === 'review') {
      homebaseUi.resetFilter()
    }
    const switching = this.workspaceId !== workspaceId
    this.view = 'workspace'
    this.scratchId = null
    this.workspaceId = workspaceId
    const list = threads.forWorkspace(workspaceId)
    this.threadId = threadId ?? list[0]?.id ?? null
    if (switching) {
      workspaceUi.resetForWorkspace()
    }
    this.requestFocus('workspace')
  }

  openScratch(scratchId: string, focusComposer = false) {
    this.view = 'scratch'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = scratchId
    this.scratchComposerFocus = focusComposer
    this.requestFocus('scratch')
  }

  requestNewScratch() {
    window.dispatchEvent(new CustomEvent('cormux:new-scratch'))
  }

  requestCommandPalette() {
    this.commandPaletteRequested = true
    window.dispatchEvent(new CustomEvent('cormux:command-palette'))
  }

  requestNewWorkspace() {
    this.newWorkspaceRequested = true
    this.newWorkspacePullRequest = null
    window.dispatchEvent(new CustomEvent('cormux:new-workspace'))
  }

  requestNewWorkspaceForPullRequest(context: {
    repoId: string | null
    repoFullName: string
    branch: string
    prNumber: number
    title: string
    mode: 'review' | 'continue'
  }) {
    this.newWorkspacePullRequest = context
    this.newWorkspaceRequested = true
    window.dispatchEvent(new CustomEvent('cormux:new-workspace'))
  }

  applyView(view: AppView) {
    this.scratchId = null
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
