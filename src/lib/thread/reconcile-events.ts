import type { AgentEvent, StreamedEvent } from '$lib/ipc/bindings'

export type TimedEvent = {
  /** The row's order on screen; matches the store for rows read from it. */
  seq: number
  atMs: number
  event: AgentEvent
  /** Where the store saved a streamed event. Unset on rows read back, and on anything unsaved. */
  storeSeq?: number | null
}

// The core saves an attachment-carrying prompt under different text than the composer shows.
function eventKey(event: AgentEvent) {
  return event.type === 'messageChunk' && event.role === 'user'
    ? 'user'
    : JSON.stringify(event)
}

/** A local event the store hasn't caught up with yet stays this long, then the store wins. */
export const UNSAVED_GRACE_MS = 5000

/**
 * Laid over what the store returned: events shown before the store confirmed them, keeping only
 * those it doesn't have yet. Streamed events are saved before they're sent, so one that's missing
 * and old was never going to be saved (a stop the UI marked itself). A streamed event that knows
 * its place in the store needs no guessing: it's dropped once read back, and kept until then.
 */
export function reconcileLocalEvents(
  saved: TimedEvent[],
  local: TimedEvent[],
  nowMs: number,
): TimedEvent[] {
  const streamed = new Set(local.map((row) => row.storeSeq))
  const savedSeqs = new Set(saved.map((row) => row.seq))
  const unmatched = new Map<string, number>()
  for (const { seq, event } of saved) {
    if (streamed.has(seq)) continue
    const key = eventKey(event)
    unmatched.set(key, (unmatched.get(key) ?? 0) + 1)
  }
  return local.filter(({ event, atMs, storeSeq }) => {
    if (storeSeq != null) return !savedSeqs.has(storeSeq)
    const key = eventKey(event)
    const count = unmatched.get(key) ?? 0
    if (count > 0) {
      unmatched.set(key, count - 1)
      return false
    }
    return nowMs - atMs < UNSAVED_GRACE_MS
  })
}

/**
 * Adds streamed events to what's shown. A read of the store can overtake the stream, so anything it
 * already brought back is skipped; events the store has no place for always show.
 */
export function appendStreamed(
  shown: TimedEvent[],
  newestSeq: number,
  batch: StreamedEvent[],
  atMs: number,
): TimedEvent[] {
  const streamed = new Set(
    shown.filter((row) => row.seq > newestSeq).map((row) => row.storeSeq),
  )
  const fresh = batch.filter(
    ({ seq }) => seq == null || (seq > newestSeq && !streamed.has(seq)),
  )
  if (fresh.length === 0) return shown
  const start = Math.max(shown.at(-1)?.seq ?? 0, newestSeq)
  return [
    ...shown,
    ...fresh.map(({ seq, event }, index) => ({
      seq: start + index + 1,
      atMs,
      event,
      storeSeq: seq,
    })),
  ]
}

/** What's shown after a read of the rows following `newestSeq`, and the new `newestSeq`. */
export function mergeNewerRows(
  shown: TimedEvent[],
  newestSeq: number,
  saved: TimedEvent[],
  fetchedNewest: number,
  nowMs: number,
): TimedEvent[] {
  const split = shown.findIndex((row) => row.seq > newestSeq)
  const base = split < 0 ? shown : shown.slice(0, split)
  const local = split < 0 ? [] : shown.slice(split)
  if (!saved.length && !local.length) return shown
  const unsaved = reconcileLocalEvents(saved, local, nowMs)
  return [
    ...base,
    ...saved,
    ...unsaved.map((row, index) => ({
      ...row,
      seq: fetchedNewest + index + 1,
    })),
  ]
}
