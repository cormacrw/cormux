import { tick } from 'svelte'

export type WorkspacePanelTab =
  'thread' | 'plan' | 'findings' | 'changes' | 'output'

/** Git and Output slide out over the panel underneath instead of replacing it. */
export type WorkspaceSlideOut = 'changes' | 'output'
export type WorkspaceBaseTab = Exclude<WorkspacePanelTab, WorkspaceSlideOut>

export function isSlideOut(tab: WorkspacePanelTab): tab is WorkspaceSlideOut {
  return tab === 'changes' || tab === 'output'
}

const PINNED_TRAY_KEY = 'cormux-pinned-tray'
const TRAY_WIDTH_KEY = 'cormux-git-pane-width'

function readTrayWidth(): number {
  try {
    const stored = Number(globalThis.localStorage?.getItem(TRAY_WIDTH_KEY))
    return stored >= 320 ? stored : 640
  } catch {
    return 640
  }
}

function readPinnedTray(): WorkspaceSlideOut | null {
  try {
    const stored = globalThis.localStorage?.getItem(PINNED_TRAY_KEY)
    return stored === 'changes' || stored === 'output' ? stored : null
  } catch {
    return null
  }
}

function writePinnedTray(tray: WorkspaceSlideOut | null) {
  try {
    globalThis.localStorage?.setItem(PINNED_TRAY_KEY, tray ?? '')
  } catch {
    // Storage is optional; the pin just won't survive a restart.
  }
}

/**
 * Runs a layout change as a view transition, so a tray glides between
 * slide-out and pinned pane, and slides away when closed (styles in app.css).
 * Plain update where unsupported or with reduced motion.
 */
function layoutTransition(update: () => void) {
  const doc = globalThis.document
  const reduced =
    doc?.documentElement.classList.contains('reduce-motion') ||
    globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  if (!doc || reduced || typeof doc.startViewTransition !== 'function') {
    update()
    return
  }
  const transition = doc.startViewTransition(async () => {
    update()
    await tick()
  })
  // A newer layout change can cut this one short; the update still lands.
  transition.ready.catch(() => {})
  transition.finished.catch(() => {})
}

export class WorkspaceUiStore {
  activeTab = $state<WorkspacePanelTab>('thread')
  /** The panel under a slide-out: the last thread, plan or findings tab. */
  baseTab = $state<WorkspaceBaseTab>('thread')
  readonly slideOut = $derived<WorkspaceSlideOut | null>(
    isSlideOut(this.activeTab) ? this.activeTab : null,
  )
  /**
   * The tray docked beside the conversation as a pane, if any. Closing a
   * pinned tray unpins it, so the next open is a slide-out again.
   */
  pinnedTray = $state<WorkspaceSlideOut | null>(readPinnedTray())
  /** Width of the pinned pane in px; dragging its edge changes it. */
  trayWidth = $state(readTrayWidth())
  /** Git is on screen, as a slide-out or as the pinned pane. */
  readonly gitVisible = $derived(
    this.activeTab === 'changes' || this.pinnedTray === 'changes',
  )
  readonly outputVisible = $derived(
    this.activeTab === 'output' || this.pinnedTray === 'output',
  )
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
    // While a tray is pinned, the other tray takes its place as the pane,
    // and the panel underneath keeps focus.
    if (isSlideOut(tab) && this.pinnedTray) {
      if (isSlideOut(this.activeTab)) this.activeTab = this.baseTab
      if (this.pinnedTray !== tab) {
        layoutTransition(() => this.pinTray(tab))
      }
      return
    }
    if (!isSlideOut(tab)) this.baseTab = tab
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

  /** ⌘G: shows or hides Git. */
  toggleGit() {
    this.toggleTray('changes')
  }

  /** ⌘O or Ctrl+`: shows or hides Output. */
  toggleOutputTab() {
    this.toggleTray('output')
  }

  toggleTray(tray: WorkspaceSlideOut) {
    const showing = tray === 'changes' ? this.gitVisible : this.outputVisible
    if (showing) this.closeTray(tray)
    else this.openTab(tray)
  }

  /** Hides a tray, whichever way it is showing; a pinned one unpins as it goes. */
  closeTray(tray: WorkspaceSlideOut) {
    if (this.pinnedTray === tray) {
      layoutTransition(() => this.pinTray(null))
      return
    }
    if (this.activeTab === tray) this.closeSlideOut()
  }

  /** Pins a tray beside the conversation, or unpins it back to a slide-out that stays open. */
  setTrayPinned(tray: WorkspaceSlideOut, pinned: boolean) {
    layoutTransition(() => {
      if (pinned) {
        this.pinTray(tray)
        if (isSlideOut(this.activeTab)) this.activeTab = this.baseTab
      } else {
        this.pinTray(null)
        this.activeTab = tray
      }
    })
  }

  private pinTray(tray: WorkspaceSlideOut | null) {
    this.pinnedTray = tray
    writePinnedTray(tray)
  }

  setTrayWidth(width: number) {
    this.trayWidth = Math.round(width)
    try {
      globalThis.localStorage?.setItem(TRAY_WIDTH_KEY, String(this.trayWidth))
    } catch {
      // Optional, as above.
    }
  }

  resetForWorkspace() {
    this.activeTab = 'thread'
    this.baseTab = 'thread'
    this.revealDiffPath = null
    this.revealDiffLine = null
    this.collapsedDiffPaths = {}
    this.planThreadId = null
  }
}

export const workspaceUi = new WorkspaceUiStore()
