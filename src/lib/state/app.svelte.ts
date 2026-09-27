import type { AppView, Snapshot } from '$lib/ipc'

export type ViewId = 'homebase' | 'workspace' | 'settings'

export class AppStore {
  version = $state(0)
  view = $state<ViewId>('homebase')
  workspaceId = $state<string | null>(null)

  readonly windowTitle = $derived.by(() => {
    if (this.view === 'settings') return 'Harness · Settings'
    if (this.view === 'workspace' && this.workspaceId) {
      return `Harness · ${this.workspaceId}`
    }
    return 'Harness · Homebase'
  })

  hydrate(snapshot: Snapshot) {
    this.version = snapshot.version
    if (snapshot.view === 'homebase') {
      this.view = 'homebase'
      this.workspaceId = null
      return
    }
    if (snapshot.view === 'settings') {
      this.view = 'settings'
      this.workspaceId = null
      return
    }
    this.view = 'workspace'
    this.workspaceId = snapshot.view.workspace.id
  }

  openHomebase() {
    this.view = 'homebase'
    this.workspaceId = null
  }

  openSettings() {
    this.view = 'settings'
    this.workspaceId = null
  }

  openWorkspace(id: string) {
    this.view = 'workspace'
    this.workspaceId = id
  }

  applyView(view: AppView) {
    this.hydrate({ version: this.version, view })
  }
}

export const app = new AppStore()
