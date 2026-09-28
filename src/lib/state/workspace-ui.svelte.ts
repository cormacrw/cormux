export type WorkspacePanelTab = 'thread' | 'findings' | 'output'

export class WorkspaceUiStore {
  activeTab = $state<WorkspacePanelTab>('thread')
  /** Tab before switching to Output (for Ctrl+` toggle). */
  tabBeforeOutput = $state<WorkspacePanelTab>('thread')
  changesOpen = $state(false)

  openTab(tab: WorkspacePanelTab) {
    if (tab === 'output') {
      if (this.activeTab !== 'output') {
        this.tabBeforeOutput = this.activeTab
      }
      this.activeTab = 'output'
      return
    }
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

  resetForWorkspace() {
    this.activeTab = 'thread'
    this.tabBeforeOutput = 'thread'
  }
}

export const workspaceUi = new WorkspaceUiStore()
