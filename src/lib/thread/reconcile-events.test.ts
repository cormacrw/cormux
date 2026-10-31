import { describe, expect, it } from 'vitest'
import type { AgentEvent } from '$lib/ipc/bindings'
import {
  appendStreamed,
  mergeNewerRows,
  reconcileLocalEvents,
  UNSAVED_GRACE_MS,
  type TimedEvent,
} from './reconcile-events'

const chunk = (text: string, role = 'agent') =>
  ({ type: 'messageChunk', role, text }) as AgentEvent
const row = (event: AgentEvent, atMs = 1000) => ({ seq: 1, atMs, event })

describe('reconcileLocalEvents', () => {
  it('drops what the store now has, once per saved row', () => {
    const kept = reconcileLocalEvents(
      [row(chunk('a'))],
      [row(chunk('a')), row(chunk('a')), row(chunk('b'))],
      1000,
    )
    expect(kept.map((entry) => entry.event)).toEqual([chunk('a'), chunk('b')])
  })

  it('matches your own message whatever text was saved', () => {
    expect(
      reconcileLocalEvents(
        [row(chunk('with attachments', 'user'))],
        [row(chunk('shown text', 'user'))],
        1000,
      ),
    ).toEqual([])
  })

  it('lets go of a local event the store never got', () => {
    expect(
      reconcileLocalEvents([], [row(chunk('a'))], 1000 + UNSAVED_GRACE_MS),
    ).toEqual([])
  })

  it('goes by the store position of a streamed event, not its contents', () => {
    const saved = [{ ...row(chunk('same')), seq: 5 }]
    const late = { ...row(chunk('same')), seq: 9, storeSeq: 5 }
    const ahead = { ...row(chunk('same')), seq: 10, storeSeq: 6 }
    expect(reconcileLocalEvents(saved, [late, ahead], 99999)).toEqual([ahead])
  })

  it('does not let a streamed row use up the match for a local one', () => {
    const saved = [{ ...row(chunk('you', 'user')), seq: 5 }]
    const local = row(chunk('you', 'user'))
    const streamed = { ...row(chunk('you', 'user')), seq: 9, storeSeq: 5 }
    expect(reconcileLocalEvents(saved, [streamed, local], 1000)).toEqual([
      local,
    ])
  })
})

describe('streamed events and store reads', () => {
  const stored = (seq: number, text: string): TimedEvent => ({
    seq,
    atMs: 1000,
    event: chunk(text),
  })
  const texts = (rows: TimedEvent[]) =>
    rows.map(({ event }) => (event as { text: string }).text)

  it('skips a late event a read already brought back', () => {
    const shown = [stored(1, 'one'), stored(2, 'two')]
    expect(
      appendStreamed(shown, 2, [{ seq: 2, event: chunk('two') }], 1000),
    ).toBe(shown)
  })

  it('skips an event delivered twice', () => {
    const once = appendStreamed(
      [stored(1, 'one')],
      1,
      [{ seq: 2, event: chunk('two') }],
      1000,
    )
    const twice = appendStreamed(
      once,
      1,
      [{ seq: 2, event: chunk('two') }],
      1000,
    )
    expect(texts(twice)).toEqual(['one', 'two'])
  })

  it('still shows an event the store failed to save', () => {
    const shown = appendStreamed(
      [stored(1, 'one')],
      1,
      [{ seq: null, event: chunk('unsaved') }],
      1000,
    )
    expect(texts(shown)).toEqual(['one', 'unsaved'])
  })

  it('trades streamed rows for saved ones, keeping order', () => {
    const shown = appendStreamed(
      [stored(1, 'one')],
      1,
      [
        { seq: 2, event: chunk('two') },
        { seq: 3, event: chunk('three') },
      ],
      1000,
    )
    const merged = mergeNewerRows(shown, 1, [stored(2, 'two')], 2, 1000)
    expect(texts(merged)).toEqual(['one', 'two', 'three'])
    expect(merged.map((row) => row.seq)).toEqual([1, 2, 3])
    const late = appendStreamed(
      merged,
      2,
      [{ seq: 3, event: chunk('three') }],
      1000,
    )
    expect(texts(late)).toEqual(['one', 'two', 'three'])
  })

  it('keeps a saved event that is older than the grace period until read', () => {
    const shown = appendStreamed(
      [stored(1, 'one')],
      1,
      [{ seq: 2, event: chunk('two') }],
      1000,
    )
    const merged = mergeNewerRows(shown, 1, [], 1, 1000 + UNSAVED_GRACE_MS * 2)
    expect(texts(merged)).toEqual(['one', 'two'])
  })

  it('numbers what stays above the rows of a first read', () => {
    const shown = appendStreamed(
      [],
      0,
      [{ seq: null, event: chunk('local') }],
      1000,
    )
    const merged = mergeNewerRows(
      shown,
      0,
      [stored(7, 'one'), stored(8, 'two')],
      8,
      1000,
    )
    expect(merged.map((row) => row.seq)).toEqual([7, 8, 9])
  })
})
