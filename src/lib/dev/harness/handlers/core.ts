import type { SettingsPatch } from '$lib/ipc/bindings'
import { threadActivity } from '../db'
import { fixtureEngines } from '../fixtures'
import { CHANGE, type HarnessRuntime } from '../runtime'
import type { HandlerSlice } from '../types'

export function coreHandlers({
  db,
  scenario,
  emitStateChanged,
}: HarnessRuntime) {
  return {
    get_snapshot: () => {
      const { snapshot } = db
      snapshot.persisted.threadActivity = threadActivity(db)
      snapshot.pendingApprovals = snapshot.persisted.approvals.filter(
        (row) => row.status === 'pending',
      ).length
      // The harness's repos have no committed config, so only their own commands count.
      snapshot.repoRunCommands = snapshot.persisted.repos.flatMap((repo) =>
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
      // change, so handing them the harness's own objects would hide its edits.
      return structuredClone(snapshot)
    },
    // There's no shell environment to read again.
    reload_environment: () => null,
    get_metrics: () => db.snapshot.memory!,
    detect_engines: () => scenario.engines ?? fixtureEngines,
    get_settings: () => db.snapshot.settings,
    update_settings: ({ patch }: { patch: SettingsPatch }) => {
      const changes = Object.fromEntries(
        Object.entries(patch).filter(([, value]) => value != null),
      )
      db.snapshot.settings = { ...db.snapshot.settings, ...changes }
      emitStateChanged(CHANGE.state)
      return db.snapshot.settings
    },
    end_scratch: ({ scratchId }: { scratchId: string }) => {
      const persisted = db.snapshot.persisted
      persisted.scratches = persisted.scratches.filter(
        (row) => row.id !== scratchId,
      )
      emitStateChanged(CHANGE.state)
      return null
    },
    // Nothing behind a subscription needs stopping: its channel just goes quiet.
    unsubscribe: () => null,
  } satisfies HandlerSlice
}
