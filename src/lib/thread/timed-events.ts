import type { StreamedEvent, TranscriptEvent } from '$lib/ipc/bindings'

export type TimedEvent = {
  /** Where the store saved it; for a signal, where the newest saved event before it is. */
  seq: number
  atMs: number
  event: TranscriptEvent
  /** Streamed but never saved, so no read of the store brings it back. */
  signal?: boolean
}

/** Saved rows laid in among what's shown, in store order, skipping any already there. */
export function mergeSaved(
  shown: TimedEvent[],
  rows: TimedEvent[],
): TimedEvent[] {
  const have = new Set(shown.flatMap((row) => (row.signal ? [] : [row.seq])))
  const fresh = rows.filter((row) => !have.has(row.seq))
  const first = fresh[0]
  if (!first) return shown
  const merged = [...shown, ...fresh]
  // Usually all newer; a page of older rows, or a read that overtook the stream, lands between.
  return first.seq > (shown.at(-1)?.seq ?? 0)
    ? merged
    : merged.sort((a, b) => a.seq - b.seq)
}

/** Streamed events added to what's shown. A saved one a read of the store already brought is skipped. */
export function appendStreamed(
  shown: TimedEvent[],
  batch: StreamedEvent[],
  atMs: number,
): TimedEvent[] {
  let newest = shown.at(-1)?.seq ?? 0
  const added: TimedEvent[] = []
  const late: TimedEvent[] = []
  for (const { seq, event } of batch) {
    if (seq == null) {
      added.push({ seq: newest, atMs, event, signal: true })
    } else if (seq > newest) {
      added.push({ seq, atMs, event })
      newest = seq
    } else {
      late.push({ seq, atMs, event })
    }
  }
  const next = added.length ? [...shown, ...added] : shown
  return late.length ? mergeSaved(next, late) : next
}
