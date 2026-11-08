import {
  fixtureEngines,
  fixtureSnapshot,
  fixtureTimeline,
} from '../../fixture-snapshot'
import type { HarnessRuntime } from '../runtime'
import type { SettingsPatch } from '$lib/ipc/bindings'
import type { HandlerSlice } from '../types'

/** Matches `thread_activity` in the core's store. */
function threadActivityRows() {
  const latest = new Map<string, string>()
  for (const row of fixtureTimeline) {
    if (
      row.kind === 'turn_end' ||
      row.kind === 'exit' ||
      (row.kind === 'message' && row.payload.includes('"role":"user"'))
    ) {
      latest.set(row.threadId, row.createdAt)
    }
  }
  return [...latest].map(([threadId, createdAt]) => ({ threadId, createdAt }))
}

export function coreHandlers({ scenario }: HarnessRuntime) {
  return {
    get_snapshot: () => {
      fixtureSnapshot.persisted.threadActivity = threadActivityRows()
      // The core counts its pending approval rows.
      fixtureSnapshot.pendingApprovals =
        fixtureSnapshot.persisted.approvals.filter(
          (row) => row.status === 'pending',
        ).length
      // The harness's repos have no committed config, so only their own commands count.
      fixtureSnapshot.repoRunCommands = fixtureSnapshot.persisted.repos.flatMap(
        (repo) =>
          repo.runCommand?.trim()
            ? [
                {
                  repoId: repo.id,
                  command: repo.runCommand.trim(),
                  source: 'repo' as const,
                },
              ]
            : [],
      )
      // A copy, as the real IPC deserialises one: the stores keep rows whose contents didn't
      // change, so handing them the fixture's own objects would hide the harness's edits.
      return structuredClone(fixtureSnapshot)
    },
    // There's no shell environment to read again.
    reload_environment: () => null,
    get_metrics: () => fixtureSnapshot.memory!,
    detect_engines: () => scenario.engines ?? fixtureEngines,
    get_settings: () => fixtureSnapshot.settings,
    update_settings: ({ patch }: { patch: SettingsPatch }) => {
      const changes = Object.fromEntries(
        Object.entries(patch).filter(([, value]) => value != null),
      )
      fixtureSnapshot.settings = { ...fixtureSnapshot.settings, ...changes }
      return fixtureSnapshot.settings
    },
    end_scratch: ({ scratchId }: { scratchId: string }) => {
      const persisted = fixtureSnapshot.persisted
      persisted.scratches = persisted.scratches.filter(
        (row) => row.id !== scratchId,
      )
      return null
    },
    // Nothing behind a subscription needs stopping: its channel just goes quiet.
    unsubscribe: () => null,
  } satisfies HandlerSlice
}
