import type { AppView, Snapshot } from '$lib/ipc'
import { workspaceUi } from './workspace-ui.svelte'
import { workspaces } from './workspaces.svelte'
import { threads } from './threads.svelte'
import { scratches } from './scratches.svelte'
import { repos } from './repos.svelte'
import { clickup } from './clickup.svelte'
import { appName } from '$lib/build-mode'
import { resolveWindowTitle, type ViewId } from './window-title'

export type { ViewId } from './window-title'
export { resolveWindowTitle } from './window-title'

export type FocusTarget =
  'newThread' | 'settings' | 'workspace' | 'scratch' | 'sprint' | 'repo'

export class AppStore {
  version = $state(0)
  view = $state<ViewId>('newThread')
  workspaceId = $state<string | null>(null)
  threadId = $state<string | null>(null)
  /** The open scratch, or null on every other view. */
  scratchId = $state<string | null>(null)
  /** The repo page, or null on every other view. */
  repoId = $state<string | null>(null)

  /** Incremented when a view should move focus to its main heading. */
  focusGeneration = $state(0)
  focusTarget = $state<FocusTarget | null>(null)

  /** Set while the palette is opening (sidebar / menu hooks). */
  commandPaletteRequested = $state(false)

  readonly windowTitle = $derived.by(() => {
    if (this.view === 'scratch') {
      return resolveWindowTitle(
        this.view,
        this.scratchId,
        this.scratchId ? scratches.getById(this.scratchId)?.title : undefined,
        appName,
      )
    }
    if (this.view === 'repo') {
      return resolveWindowTitle(
        this.view,
        this.repoId,
        this.repoId ? repos.getById(this.repoId)?.name : undefined,
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
    // A snapshot read before a newer event must not look like a gap on the next one.
    this.version = Math.max(this.version, snapshot.version)
  }

  /** The first page: an empty thread that becomes a workspace on its first message. */
  openNewThread() {
    this.view = 'newThread'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    this.repoId = null
    this.requestFocus('newThread')
  }

  /** The ClickUp sprint board, optionally with one task open in the side pane. */
  openSprint(taskId?: string) {
    this.view = 'sprint'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    this.repoId = null
    if (taskId) clickup.select(taskId)
    this.requestFocus('sprint')
  }

  openSettings() {
    if (this.view === 'settings') return
    this.view = 'settings'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    this.repoId = null
    this.requestFocus('settings')
  }

  openWorkspace(workspaceId: string, threadId?: string) {
    const switching = this.workspaceId !== workspaceId
    this.view = 'workspace'
    this.scratchId = null
    this.repoId = null
    this.workspaceId = workspaceId
    this.threadId = threadId ?? threads.liveFor(workspaceId)?.id ?? null
    if (switching) {
      workspaceUi.resetForWorkspace()
    }
    this.requestFocus('workspace')
  }

  openScratch(scratchId: string) {
    this.view = 'scratch'
    this.workspaceId = null
    this.threadId = null
    this.repoId = null
    this.scratchId = scratchId
    this.requestFocus('scratch')
  }

  openRepo(repoId: string) {
    this.view = 'repo'
    this.workspaceId = null
    this.threadId = null
    this.scratchId = null
    this.repoId = repoId
    this.requestFocus('repo')
  }

  requestCommandPalette() {
    this.commandPaletteRequested = true
    window.dispatchEvent(new CustomEvent('cormux:command-palette'))
  }

  applyView(view: AppView) {
    this.scratchId = null
    this.repoId = null
    if (view === 'homebase') {
      this.view = 'newThread'
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
