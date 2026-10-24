export type WorkspacePanelTab =
  'thread' | 'plan' | 'findings' | 'changes' | 'output'

/** Git and Output slide out over the panel underneath instead of replacing it. */
export type WorkspaceSlideOut = 'changes' | 'output'
export type WorkspaceBaseTab = Exclude<WorkspacePanelTab, WorkspaceSlideOut>

export function isSlideOut(tab: WorkspacePanelTab): tab is WorkspaceSlideOut {
  return tab === 'changes' || tab === 'output'
}

export class WorkspaceUiStore {
  activeTab = $state<WorkspacePanelTab>('thread')
  /** The panel under a slide-out: the last thread, plan or findings tab. */
  baseTab = $state<WorkspaceBaseTab>('thread')
  readonly slideOut = $derived<WorkspaceSlideOut | null>(
    isSlideOut(this.activeTab) ? this.activeTab : null,
  )
  /** Tab before switching to Output (for Ctrl+` toggle). */
  tabBeforeOutput = $state<WorkspacePanelTab>('thread')
  /** File to expand and scroll to in Changes (set by links in the conversation). */
  revealDiffPath = $state<string | null>(null)
  /** Line of the new file to scroll to once `revealDiffPath` is shown. */
  revealDiffLine = $state<number | null>(null)
  /** Files the user folded (true) or unfolded (false) in Changes; others use the default. */
  collapsedDiffPaths = $state<Record<string, boolean>>({})
  diffMode = $state<'unified' | 'split'>('unified')
  /** When true, opening Findings focuses the panel heading (card entry). */
  findingsFocusPending = $state(false)
  /** The thread whose plan the Plan tab shows; null shows the first one waiting. */
  planThreadId = $state<string | null>(null)

  openTab(tab: WorkspacePanelTab) {
    if (!isSlideOut(tab)) this.baseTab = tab
    if (tab === 'output') {
      if (this.activeTab !== 'output') {
        this.tabBeforeOutput = this.activeTab
      }
      this.activeTab = 'output'
      return
    }
    this.activeTab = tab
  }

  /** Opens the Plan tab on a thread's newest plan. */
  openPlan(threadId: string) {
    this.planThreadId = threadId
    this.openTab('plan')
  }

  /** Opens Changes at a file, and at a line of it when given. */
  revealInChanges(path: string, line: number | null = null) {
    this.revealDiffPath = path
    this.revealDiffLine = line
    this.openTab('changes')
  }

  /** Slides Git or Output away, back to the panel underneath. */
  closeSlideOut() {
    if (isSlideOut(this.activeTab)) this.activeTab = this.baseTab
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
    this.baseTab = 'thread'
    this.tabBeforeOutput = 'thread'
    this.revealDiffPath = null
    this.revealDiffLine = null
    this.collapsedDiffPaths = {}
    this.planThreadId = null
  }
}

export const workspaceUi = new WorkspaceUiStore()
