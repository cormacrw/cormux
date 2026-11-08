import {
  commands,
  events,
  type Snapshot,
  type StateChanged,
} from '$lib/ipc/bindings'
import { coalesce } from './coalesce'

type Change = StateChanged['change']

/**
 * Keeps the stores in step with the core. The core owns the truth and the webview re-reads a
 * whole snapshot after every state event, so this is the only place that fetches or applies
 * one: stores hand it their slice with `onSnapshot` and never see each other's.
 */
export function createSync(fetch: () => Promise<Snapshot>) {
  const slices = new Set<(snapshot: Snapshot) => void>()
  const changeListeners = new Set<(change: Change | 'gap') => void>()
  let applied: Snapshot | null = null
  let seenVersion = 0

  function apply(snapshot: Snapshot) {
    // A fetch that started before a newer snapshot landed must not undo it.
    if (applied && snapshot.version < applied.version) return
    applied = snapshot
    seenVersion = Math.max(seenVersion, snapshot.version)
    for (const slice of slices) slice(snapshot)
  }

  /** Fetch and apply a snapshot. Calls that land during one share the next, not one each. */
  const refreshSnapshot = coalesce(async () => apply(await fetch()))

  /** One state event: the core moved, so re-read; a skipped version means one was missed. */
  function receive(event: StateChanged) {
    if (event.version > seenVersion + 1) {
      for (const listener of changeListeners) listener('gap')
    } else {
      seenVersion = Math.max(seenVersion, event.version)
      for (const listener of changeListeners) listener(event.change)
    }
    void refreshSnapshot()
  }

  return {
    refreshSnapshot,
    receive,
    /**
     * Applies `slice` to every snapshot from now on, starting with the last one if a store
     * registers late (a lazily loaded view's).
     */
    onSnapshot(slice: (snapshot: Snapshot) => void) {
      slices.add(slice)
      if (applied) slice(applied)
      return () => void slices.delete(slice)
    },
    /** What a state event says changed besides the snapshot; `gap` after a missed one. */
    onChange(listener: (change: Change | 'gap') => void) {
      changeListeners.add(listener)
      return () => void changeListeners.delete(listener)
    },
  }
}

async function fetchSnapshot(): Promise<Snapshot> {
  const result = await commands.getSnapshot()
  if (result.status === 'error') {
    throw new Error(`get_snapshot failed: ${JSON.stringify(result.error)}`)
  }
  return result.data
}

const sync = createSync(fetchSnapshot)

export const { refreshSnapshot, onSnapshot, onChange } = sync

/** Listen for the core's state events; resolves to the function that stops listening. */
export function listenForStateChanges() {
  return events.stateChanged.listen((event) => sync.receive(event.payload))
}
