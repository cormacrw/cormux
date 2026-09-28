import type { HomebaseFilter } from '$lib/homebase/filter'
import type { Workspace } from './workspaces.svelte'

export class HomebaseUiStore {
  filter = $state<HomebaseFilter>('all')
  /** Cards that have already been shown (skip enter animation). */
  seenCardIds = $state<string[]>([])
  exitingCards = $state<Workspace[]>([])
  /** Bumps every 60s while Homebase is visible. */
  ageTick = $state(0)

  resetFilter() {
    this.filter = 'all'
  }

  markCardSeen(id: string) {
    if (this.seenCardIds.includes(id)) return
    this.seenCardIds = [...this.seenCardIds, id]
  }

  beginCardExit(workspace: Workspace) {
    if (this.exitingCards.some((item) => item.id === workspace.id)) return
    this.exitingCards = [...this.exitingCards, workspace]
  }

  finishCardExit(id: string) {
    this.exitingCards = this.exitingCards.filter((item) => item.id !== id)
    this.seenCardIds = this.seenCardIds.filter((item) => item !== id)
  }

  bumpAgeTick() {
    this.ageTick += 1
  }
}

export const homebaseUi = new HomebaseUiStore()
