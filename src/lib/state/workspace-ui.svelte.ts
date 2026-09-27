export type WorkspacePanelTab = 'thread' | 'findings' | 'output'

export class WorkspaceUiStore {
  activeTab = $state<WorkspacePanelTab>('thread')
  /** Tab before switching to Output (for Ctrl+` toggle). */
  tabBeforeOutput = $state<WorkspacePanelTab>('thread')
  changesOpen = $state(false)

  openTab(tab: WorkspacePanelTab) {
    this.activeTab = tab
  }

  toggleOutputTab() {
    if (this.activeTab === 'output') {
      this.activeTab = this.tabBeforeOutput
      return
    }
    this.tabBeforeOutput = this.activeTab
    this.activeTab = 'output'
  }
}

export const workspaceUi = new WorkspaceUiStore()
