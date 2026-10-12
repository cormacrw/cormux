import { commands } from '$lib/ipc'
import { showToast } from '$lib/feedback/show-toast'
import { plans } from '$lib/state/plans.svelte'
import { settings } from '$lib/state/settings.svelte'
import { threads } from '$lib/state/threads.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { hiddenPlansKey, planModeKey, threadPlanMode } from './plans'

/**
 * Switches a thread in or out of plan mode. Leaving it drops a plan still waiting for review,
 * and the agent carries on in normal mode. Resolves false (after toasting) if it didn't switch.
 */
export async function setThreadPlanMode(
  threadId: string,
  on: boolean,
): Promise<boolean> {
  settings.applyLocal(planModeKey(threadId), String(on))
  const result = await commands.setThreadPlanMode(threadId, on)
  if (result.status === 'ok') return true
  settings.applyLocal(planModeKey(threadId), String(!on))
  showToast({
    tone: 'bad',
    parts: [
      {
        type: 'text',
        value:
          typeof result.error.message === 'string'
            ? result.error.message
            : 'Could not switch plan mode',
      },
    ],
  })
  return false
}

/** Closing the Plan tab hides its plans and takes the workspace's threads out of plan mode. */
export async function closePlanTab(workspaceId: string) {
  const threadIds = threads.forWorkspace(workspaceId).map((thread) => thread.id)
  const shown = plans.visibleFor(workspaceId, threadIds).map((plan) => plan.id)
  // Remember only plans that still exist, so the setting doesn't grow forever.
  const hidden = [...plans.hiddenIds(workspaceId), ...shown].filter((id) =>
    plans.byId(id),
  )
  workspaceUi.openTab('thread')
  void settings.persist(hiddenPlansKey(workspaceId), JSON.stringify(hidden))
  await Promise.all(
    threadIds
      .filter((threadId) => threadPlanMode(settings.rows, threadId))
      .map((threadId) => setThreadPlanMode(threadId, false)),
  )
}
