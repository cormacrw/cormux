import {
  fixtureEngines,
  fixtureSnapshot,
  fixtureTimeline,
} from '../../fixture-snapshot'
import type { HarnessRuntime } from '../runtime'
import type { HandlerSlice, Input } from '../types'

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
      // A copy, as the real IPC deserialises one: the stores keep rows whose contents didn't
      // change, so handing them the fixture's own objects would hide the harness's edits.
      return structuredClone(fixtureSnapshot)
    },
    // There's no shell environment to read again.
    reload_environment: () => null,
    get_metrics: () => fixtureSnapshot.memory!,
    detect_engines: () => scenario.engines ?? fixtureEngines,
    set_setting: ({ input: { key, value } }: Input<'set_setting'>) => {
      const persisted = fixtureSnapshot.persisted
      persisted.settings = [
        ...persisted.settings.filter((row) => row.key !== key),
        { key, value },
      ]
      return null
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
