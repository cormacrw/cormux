import { commands } from '$lib/ipc'
import type { Workspace } from '$lib/state/workspaces.svelte'

const pending = new Set<string>()

export function localSummaryFallback(workspace: Workspace): string {
  if (workspace.kind === 'review' && workspace.prNumber != null) {
    return `Reviewing #${workspace.prNumber} on branch ${workspace.branch}. An agent is reading the diff and will draft comments for your approval; nothing has been posted to GitHub yet.`
  }
  return `Just started on “${workspace.name}”, branched as ${workspace.branch}. Work is getting underway; no summary from the model yet.`
}

export async function ensureWorkspaceSummary(
  workspace: Workspace,
  onUpdate: (patch: {
    summary: string
    summaryAtMs: number
    summarySource: string
  }) => void,
): Promise<void> {
  if (workspace.summary?.trim()) return
  if (pending.has(workspace.id)) return
  pending.add(workspace.id)
  try {
    const result = await commands.summariseWorkspace(workspace.id)
    if (result.status === 'ok') {
      const at = Date.parse(result.data.summaryAt)
      onUpdate({
        summary: result.data.summary,
        summaryAtMs: Number.isNaN(at) ? Date.now() : at,
        summarySource: result.data.summarySource,
      })
      return
    }
    onUpdate({
      summary: localSummaryFallback(workspace),
      summaryAtMs: Date.now(),
      summarySource: 'Haiku 4.5',
    })
  } catch {
    onUpdate({
      summary: localSummaryFallback(workspace),
      summaryAtMs: Date.now(),
      summarySource: 'Haiku 4.5',
    })
  } finally {
    pending.delete(workspace.id)
  }
}
