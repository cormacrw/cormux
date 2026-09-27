import { commands, events, type Snapshot, type StateChanged } from './bindings'

export {
  commands,
  events,
  type AgentChunk,
  type AppView,
  type DiffUpdate,
  type Error as CoreError,
  type PtyChunk,
  type Result,
  type Snapshot,
  type StateChangeKind,
  type StateChanged,
} from './bindings'

export { subscribeAgentChunks, subscribeDiffs, subscribePty } from './channels'

/** True when the UI missed at least one state event and must replace local state. */
export function hasVersionGap(lastSeen: number, incoming: number): boolean {
  return incoming > lastSeen + 1
}

function unwrapSnapshot(
  result: Awaited<ReturnType<typeof commands.getSnapshot>>,
): Snapshot {
  if (result.status === 'error') {
    throw new Error(`get_snapshot failed: ${JSON.stringify(result.error)}`)
  }
  return result.data
}

/** Fetch a full core snapshot. Call on boot, after a version gap, and after reload. */
export async function fetchSnapshot(): Promise<Snapshot> {
  return unwrapSnapshot(await commands.getSnapshot())
}

/**
 * Listen for low-volume state events. A version gap triggers a snapshot refetch
 * so a webview reload or a dropped event always recovers.
 */
export function listenForStateChanges(options: {
  lastVersion: () => number
  onEvent: (payload: StateChanged) => void
  onSnapshot: (snapshot: Snapshot) => void
}) {
  return events.stateChanged.listen(async (event) => {
    const payload = event.payload
    if (hasVersionGap(options.lastVersion(), payload.version)) {
      options.onSnapshot(await fetchSnapshot())
      return
    }
    options.onEvent(payload)
  })
}
