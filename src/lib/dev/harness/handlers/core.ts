import type { SettingsPatch } from '$lib/ipc/bindings'
import { threadActivity } from '../db'
import { fixtureEngines } from '../fixtures'
import { CHANGE, type HarnessRuntime } from '../runtime'
import type { HandlerSlice } from '../types'

export function coreHandlers({ db, emitStateChanged }: HarnessRuntime) {
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
      // As the core's `workspace_run_commands`: the workspace's own, else its repo's.
      snapshot.workspaceRunCommands = snapshot.persisted.workspaces.flatMap(
        (row) => {
          const own = row.runCommand?.trim()
          const repo = snapshot.repoRunCommands.find(
            (run) => run.repoId === row.repoId,
          )
          const run = own
            ? { command: own, source: 'workspace' as const }
            : repo && { command: repo.command, source: repo.source }
          return run ? [{ workspaceId: row.id, ...run }] : []
        },
      )
      // As the core's `repos::removal_block`.
      const { repos, workspaces } = snapshot.persisted
      snapshot.repoRemovalBlocks = repos.flatMap((repo) => {
        const used = workspaces.filter((row) => row.repoId === repo.id).length
        const reason = used
          ? `${repo.name} has ${used} workspace${used === 1 ? '' : 's'}. Tear ${used === 1 ? 'it' : 'them'} down first.`
          : repos.length <= 1
            ? 'Cormux needs at least one repo'
            : null
        return reason ? [{ repoId: repo.id, reason }] : []
      })
      // A copy, as the real IPC deserialises one: the stores keep rows whose contents didn't
      // change, so handing them the harness's own objects would hide its edits.
      return structuredClone(snapshot)
    },
    // There's no shell environment to read again.
    reload_environment: () => null,
    detect_engines: () => fixtureEngines,
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
