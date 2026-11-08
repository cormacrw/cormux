import { describe, expect, it } from 'vitest'
import type { TranscriptEvent } from '$lib/ipc/bindings'
import { appendStreamed, mergeSaved, type TimedEvent } from './timed-events'

const said = (text: string): TranscriptEvent => ({
  type: 'messageChunk',
  role: 'agent',
  text,
})
const row = (seq: number): TimedEvent => ({
  seq,
  atMs: seq,
  event: said(`row ${seq}`),
})
const seqs = (rows: TimedEvent[]) => rows.map((entry) => entry.seq)

describe('timed events', () => {
  it('skips a streamed event a read of the store already brought', () => {
    const shown = [row(1), row(2)]
    const next = appendStreamed(
      shown,
      [
        { seq: 2, event: said('row 2') },
        { seq: 3, event: said('row 3') },
      ],
      10,
    )
    expect(seqs(next)).toEqual([1, 2, 3])
  })

  it('skips rows a read brings that the stream already did', () => {
    const shown = appendStreamed([], [{ seq: 3, event: said('row 3') }], 10)
    expect(seqs(mergeSaved(shown, [row(1), row(2), row(3)]))).toEqual([1, 2, 3])
  })

  it('puts a signal after the newest saved event', () => {
    const exited: TranscriptEvent = { type: 'engineExited', code: null }
    const next = appendStreamed(
      [row(1), row(2)],
      [{ seq: null, event: exited }],
      10,
    )
    expect(next.at(-1)).toMatchObject({ seq: 2, signal: true, event: exited })
    expect(seqs(mergeSaved(next, [row(3)]))).toEqual([1, 2, 2, 3])
  })

  it('merges a page of older rows in order', () => {
    expect(
      seqs(mergeSaved([row(4), row(5)], [row(1), row(2), row(3)])),
    ).toEqual([1, 2, 3, 4, 5])
  })

  it('hands back what was shown when nothing is new', () => {
    const shown = [row(1)]
    expect(mergeSaved(shown, [row(1)])).toBe(shown)
  })
})
