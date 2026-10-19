import { parseApprovalPayload } from '$lib/approvals/payload'
import type { ApprovalRow, SettingRow } from '$lib/ipc/bindings'

/**
 * Pending: waiting on the user. Revised: the user sent comments back. Dismissed: the user
 * left plan mode without answering it.
 */
export type PlanState = 'pending' | 'approved' | 'revised' | 'dismissed'

export type Plan = {
  /** The approval id of the agent's request to leave plan mode. */
  id: string
  threadId: string
  title: string
  markdown: string
  state: PlanState
  /** What the user sent back on a revised plan. */
  feedback: string | null
  resolvedAtMs: number | null
  /** 1 for the thread's first plan, then one more per revision. */
  version: number
}

export function planModeKey(threadId: string): string {
  return `thread:${threadId}/planMode`
}

/** Plans the user closed the Plan tab on, which keeps them out of it. */
export function hiddenPlansKey(workspaceId: string): string {
  return `workspace:${workspaceId}/hiddenPlans`
}

export function hiddenPlanIds(
  settings: SettingRow[],
  workspaceId: string,
): string[] {
  const raw = settings.find((row) => row.key === hiddenPlansKey(workspaceId))
  try {
    const ids: unknown = JSON.parse(raw?.value ?? '[]')
    return Array.isArray(ids) ? ids.filter((id) => typeof id === 'string') : []
  } catch {
    return []
  }
}

/** Whether the user switched the thread to plan mode (the core keeps the same setting). */
export function threadPlanMode(
  settings: SettingRow[],
  threadId: string,
): boolean {
  const key = planModeKey(threadId)
  return settings.some((row) => row.key === key && row.value === 'true')
}

/** Every plan an agent handed over, oldest first (approvals arrive in the order they were asked). */
export function plansFromApprovals(approvals: ApprovalRow[]): Plan[] {
  const versions: Record<string, number> = {}
  const plans: Plan[] = []
  for (const row of approvals) {
    const payload = parseApprovalPayload(row.payload)
    if (payload?.effect !== 'execute_plan' || !payload.plan) continue
    const version = (versions[row.threadId] ?? 0) + 1
    versions[row.threadId] = version
    plans.push({
      id: row.id,
      threadId: row.threadId,
      title: payload.what || 'Plan',
      markdown: payload.plan,
      state:
        row.status === 'approved'
          ? 'approved'
          : row.status === 'denied'
            ? 'revised'
            : row.status === 'dismissed'
              ? 'dismissed'
              : 'pending',
      feedback: payload.denyReason ?? null,
      resolvedAtMs: payload.resolvedAtMs ?? null,
      version,
    })
  }
  return plans
}

/** Each thread's newest plan, in the threads' order. */
export function latestPlanPerThread(
  plans: Plan[],
  threadIds: string[],
): Plan[] {
  return threadIds.flatMap((threadId) => {
    const plan = plans.findLast((entry) => entry.threadId === threadId)
    return plan ? [plan] : []
  })
}
