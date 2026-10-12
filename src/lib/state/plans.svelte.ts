import type { ApprovalRow } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import type { PlanComment } from '$lib/plans/feedback'
import { settings } from './settings.svelte'
import {
  hiddenPlanIds,
  latestPlanPerThread,
  plansFromApprovals,
  type Plan,
} from '$lib/plans/plans'

/** Plans agents handed over for approval, and the user's draft comments on each. */
export class PlansStore {
  items = $state<Plan[]>([])
  /** Unsent comments, keyed by plan id. */
  comments = $state<Record<string, PlanComment[]>>({})
  /** The unsent overall note, keyed by plan id. */
  notes = $state<Record<string, string>>({})
  /** The newest plan to arrive while the app was open; the workspace view opens it. */
  arrived = $state<Plan | null>(null)
  /** Pending plan ids as of the last snapshot; not state, nothing renders it. */
  private seenPending: string[] | null = null

  hydrate(approvals: ApprovalRow[]) {
    const plans = plansFromApprovals(approvals)
    this.items = plans
    const pending = plans.filter((plan) => plan.state === 'pending')
    // Plans already waiting at launch don't count as arriving.
    if (this.seenPending) {
      const seen = this.seenPending
      const fresh = pending.filter((plan) => !seen.includes(plan.id))
      if (fresh.length) this.arrived = fresh[fresh.length - 1]!
    }
    this.seenPending = pending.map((plan) => plan.id)
  }

  byId(id: string): Plan | undefined {
    return this.items.find((plan) => plan.id === id)
  }

  /** Each of the workspace's threads' newest plan. */
  latestFor(threadIds: string[]): Plan[] {
    return latestPlanPerThread(this.items, threadIds)
  }

  hiddenIds(workspaceId: string): string[] {
    return hiddenPlanIds(settings.rows, workspaceId)
  }

  /** The plans the Plan tab shows: not dismissed, and not closed by the user. */
  visibleFor(workspaceId: string, threadIds: string[]): Plan[] {
    const hidden = this.hiddenIds(workspaceId)
    return this.latestFor(threadIds).filter(
      (plan) => plan.state !== 'dismissed' && !hidden.includes(plan.id),
    )
  }

  pendingCount(workspaceId: string, threadIds: string[]): number {
    return this.visibleFor(workspaceId, threadIds).filter(
      (plan) => plan.state === 'pending',
    ).length
  }

  commentsFor(planId: string): PlanComment[] {
    return this.comments[planId] ?? []
  }

  addComment(planId: string, comment: Omit<PlanComment, 'id'>) {
    this.comments = {
      ...this.comments,
      [planId]: [
        ...this.commentsFor(planId),
        { ...comment, id: crypto.randomUUID() },
      ],
    }
  }

  removeComment(planId: string, id: string) {
    this.comments = {
      ...this.comments,
      [planId]: this.commentsFor(planId).filter((comment) => comment.id !== id),
    }
  }

  setNote(planId: string, note: string) {
    this.notes = { ...this.notes, [planId]: note }
  }

  clearDrafts(planId: string) {
    this.comments = Object.fromEntries(
      Object.entries(this.comments).filter(([id]) => id !== planId),
    )
    this.notes = Object.fromEntries(
      Object.entries(this.notes).filter(([id]) => id !== planId),
    )
  }

  /** Approves the plan, or with `feedback` sends it back for the agent to revise. */
  async resolve(planId: string, feedback: string | null) {
    const result = await commands.resolveApproval(
      planId,
      feedback === null,
      feedback,
    )
    if (result.status === 'error') {
      throw new Error(
        typeof result.error.message === 'string'
          ? result.error.message
          : 'Could not reach the agent',
      )
    }
    this.clearDrafts(planId)
  }
}

export const plans = new PlansStore()
