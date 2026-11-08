import { WORKING_THREAD_STATUSES, type SettingsPatch } from '$lib/ipc/bindings'
import { bangName } from '$lib/bangs/bangs'
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
      // As the core's `workspaces::branch_lock`.
      snapshot.workspaceBranchLocks = snapshot.workspaces.flatMap((record) => {
        const working = snapshot.persisted.threads.some(
          (thread) =>
            thread.workspaceId === record.id &&
            (WORKING_THREAD_STATUSES as readonly string[]).includes(
              thread.status,
            ),
        )
        const reason =
          record.status === 'provisioning'
            ? 'Wait for setup to finish to change branches.'
            : working
              ? 'Pause or stop the running agents to change branches.'
              : null
        return reason ? [{ workspaceId: record.id, reason }] : []
      })
      // As the core's `review::review_block`.
      snapshot.workspaceReviewBlocks = snapshot.workspaces.flatMap((record) => {
        const lead = snapshot.persisted.threads.find(
          (thread) =>
            thread.workspaceId === record.id &&
            thread.role != null &&
            !thread.retired,
        )
        const reason =
          record.kind === 'review'
            ? 'This is a review workspace; its Reviewer already reviews the PR.'
            : record.status === 'provisioning'
              ? 'Wait for setup to finish to review.'
              : record.status === 'provisioningFailed'
                ? 'Setup failed. Retry or skip it to review.'
                : record.status === 'tearingDown'
                  ? 'This workspace is being torn down.'
                  : !lead
                    ? 'This workspace has no Lead to review with.'
                    : (WORKING_THREAD_STATUSES as readonly string[]).includes(
                          lead.status,
                        )
                      ? "The agent is still working; review once it's done."
                      : null
        return reason ? [{ workspaceId: record.id, reason }] : []
      })
      // A copy, as the real IPC deserialises one: the stores keep rows whose contents didn't
      // change, so handing them the harness's own objects would hide its edits.
      return structuredClone(snapshot)
    },
    // There's no shell environment to read again.
    reload_environment: () => null,
    detect_engines: () => fixtureEngines,
    update_settings: ({ patch }: { patch: SettingsPatch }) => {
      // A stand-in for the core's bang rules: tidied names, there and unique.
      if (patch.bangs) {
        patch = {
          ...patch,
          bangs: patch.bangs.map((bang) => ({
            ...bang,
            name: bangName(bang.name),
          })),
        }
        const names = new Set<string>()
        for (const { name } of patch.bangs ?? []) {
          if (!name) {
            throw {
              kind: 'Invalid',
              message: 'Give the bang a name, like test or review.',
            }
          }
          if (names.has(name)) {
            throw { kind: 'Invalid', message: `!${name} already exists.` }
          }
          names.add(name)
        }
      }
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
