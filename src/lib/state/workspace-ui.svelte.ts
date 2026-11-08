import { tick } from 'svelte'
import { nav } from '$lib/core/nav.svelte'
import type { WorkspacePanel } from '$lib/core/route'
import { readPaneWidth, savePaneWidth } from '$lib/primitives/slide-out'
import { readPref, writePref } from '$lib/primitives/local-pref'
import { prefersReducedMotion } from '$lib/motion'

/** Everything but the conversation slides out over it instead of replacing it. */
type WorkspaceSlideOut = Exclude<WorkspacePanel, 'thread'>
/** Git and Output can also be pinned beside the conversation. */
export type PinnableTray = 'changes' | 'output'

function isSlideOut(tab: WorkspacePanel): tab is WorkspaceSlideOut {
  return tab !== 'thread'
}

function isPinnable(tab: WorkspacePanel): tab is PinnableTray {
  return tab === 'changes' || tab === 'output'
}

const PINNED_TRAY_KEY = 'cormux-pinned-tray'
// Named before Output became a tray too; kept so saved widths still apply.
const TRAY_WIDTH_KEY = 'cormux-git-pane-width'

function readPinnedTray(): PinnableTray | null {
  const stored = readPref(PINNED_TRAY_KEY)
  return stored === 'changes' || stored === 'output' ? stored : null
}

/**
 * Runs a layout change as a view transition, so a tray glides between
 * slide-out and pinned pane, and slides away when closed (styles in app.css).
 * Plain update where unsupported or with reduced motion.
 */
function layoutTransition(update: () => void) {
  const doc = globalThis.document
  if (
    !doc ||
    prefersReducedMotion() ||
    typeof doc.startViewTransition !== 'function'
  ) {
    update()
    return
  }
  // Blurring behind moving glass every frame drops the slide to a crawl, so the
  // glass goes without its blur until the panes settle.
  const root = doc.documentElement
  root.classList.add('layout-shift')
  const transition = doc.startViewTransition(async () => {
    update()
    await tick()
  })
  latestTransition = transition
  // A newer layout change can cut this one short; the update still lands.
  transition.ready.catch(() => {})
  transition.finished
    .catch(() => {})
    .finally(() => {
      if (latestTransition === transition) root.classList.remove('layout-shift')
    })
}

let latestTransition: ViewTransition | null = null

class WorkspaceUiStore {
  activeTab = $state<WorkspacePanel>('thread')
  readonly slideOut = $derived<WorkspaceSlideOut | null>(
    isSlideOut(this.activeTab) ? this.activeTab : null,
  )
  /**
   * The tray docked beside the conversation as a pane, if any. Closing a
   * pinned tray unpins it, so the next open is a slide-out again.
   */
  pinnedTray = $state<PinnableTray | null>(readPinnedTray())
  /** Width of the pinned pane in px; dragging its edge changes it. */
  trayWidth = $state(readPaneWidth(TRAY_WIDTH_KEY, 320, 640))
  readonly gitVisible = $derived(this.visible('changes'))
  readonly outputVisible = $derived(this.visible('output'))

  openTab(tab: WorkspacePanel) {
    // While a tray is pinned, the other tray takes its place as the pane,
    // and the panel underneath keeps focus.
    if (isPinnable(tab) && this.pinnedTray) {
      if (isSlideOut(this.activeTab)) this.activeTab = 'thread'
      if (this.pinnedTray !== tab) {
        layoutTransition(() => this.pinTray(tab))
      }
      return
    }
    this.activeTab = tab
  }

  /** Slides whatever is out away, back to the conversation. */
  closeSlideOut() {
    this.activeTab = 'thread'
  }

  /** The tray is on screen, as a slide-out or as the pinned pane. */
  visible(tray: WorkspaceSlideOut) {
    return this.activeTab === tray || this.pinnedTray === tray
  }

  toggleTray(tray: WorkspaceSlideOut) {
    if (this.visible(tray)) this.closeTray(tray)
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
  setTrayPinned(tray: PinnableTray, pinned: boolean) {
    layoutTransition(() => {
      if (pinned) {
        this.pinTray(tray)
        if (isSlideOut(this.activeTab)) this.activeTab = 'thread'
      } else {
        this.pinTray(null)
        this.activeTab = tray
      }
    })
  }

  private pinTray(tray: PinnableTray | null) {
    this.pinnedTray = tray
    writePref(PINNED_TRAY_KEY, tray)
  }

  setTrayWidth(width: number) {
    this.trayWidth = Math.round(width)
    savePaneWidth(TRAY_WIDTH_KEY, this.trayWidth)
  }

  resetForWorkspace() {
    this.activeTab = 'thread'
  }
}

export const workspaceUi = new WorkspaceUiStore()

// Another workspace opens on its conversation, unless the route names a panel.
nav.onNavigate((to, from) => {
  if (to.view !== 'workspace') return
  if (from.view !== 'workspace' || from.workspaceId !== to.workspaceId) {
    workspaceUi.resetForWorkspace()
  }
  if (to.panel) workspaceUi.openTab(to.panel)
})
