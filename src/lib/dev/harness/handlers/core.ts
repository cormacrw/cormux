import type { Bang, SettingsPatch } from '$lib/ipc/bindings'
import { threadActivity } from '../db'
import { fixtureEngines } from '../fixtures'
import { CHANGE, type HarnessRuntime } from '../runtime'
import type { HandlerSlice } from '../types'

export function coreHandlers({
  db,
  scenario,
  emitStateChanged,
}: HarnessRuntime) {
  /** The bangs as the scenario says the core saves them, or its refusal. */
  const savedBangs = (bangs: Bang[]): Bang[] => {
    const sent = bangs.map((bang) => bang.name)
    const save = scenario.bangSaves?.find(
      ({ names }) =>
        names.length === sent.length &&
        names.every((name, index) => name === sent[index]),
    )
    if (!save) return bangs
    if ('error' in save.reply) {
      throw { kind: 'Invalid', message: save.reply.error }
    }
    const { names } = save.reply
    return bangs.map((bang, index) => ({ ...bang, name: names[index]! }))
  }

  return {
    get_snapshot: () => {
      const { snapshot } = db
      snapshot.persisted.threadActivity = threadActivity(db)
      snapshot.pendingApprovals = snapshot.persisted.approvals.filter(
        (row) => row.status === 'pending',
      ).length
      // A copy, as the real IPC deserialises one: the stores keep rows whose contents didn't
      // change, so handing them the harness's own objects would hide its edits.
      return structuredClone(snapshot)
    },
    // There's no shell environment to read again.
    reload_environment: () => null,
    detect_engines: () => fixtureEngines,
    update_settings: ({ patch }: { patch: SettingsPatch }) => {
      if (patch.bangs) patch = { ...patch, bangs: savedBangs(patch.bangs) }
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
