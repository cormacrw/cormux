import { describe, expect, it } from 'vitest'
import type { EditVerb, TranscriptEvent } from '$lib/ipc/bindings'
import { TimelineBuilder } from './map-events-to-timeline'
import { buildTimelineRows } from './timeline-rows'
import { splitConversations } from './conversations'
import { reuseRows } from '$lib/state/reuse-rows'

type Timed = { seq: number; atMs: number; event: TranscriptEvent }
function toolCall(id: string, edit: boolean): TranscriptEvent {
  const verb: EditVerb = 'Edited'
  return {
    type: 'toolCall',
    id,
    title: `Read src/lib/file-${id}.ts`,
    role: edit ? { kind: 'edit', verb } : { kind: 'read' },
    status: 'completed',
    locations: [`src/lib/file-${id}.ts`],
    detail: null,
  }
}

/** A thread of `turns` turns, each: a prompt, a thought, 20 tool steps, a reply, a turn end. */
export function syntheticThread(turns: number): Timed[] {
  const events: Timed[] = []
  let seq = 0
  const push = (event: TranscriptEvent) => {
    seq += 1
    events.push({ seq, atMs: 1_700_000_000_000 + seq * 1000, event })
  }
  for (let turn = 0; turn < turns; turn += 1) {
    push({ type: 'messageChunk', role: 'user', text: `Prompt ${turn}` })
    push({
      type: 'messageChunk',
      role: 'thought',
      text: 'Thinking about it. '.repeat(20),
    })
    for (let step = 0; step < 20; step += 1) {
      const id = `t${turn}-s${step}`
      push(toolCall(id, step % 5 === 0))
      push({
        type: 'toolCallUpdate',
        id,
        title: null,
        status: 'completed',
        locations: [],
        output: null,
      })
    }
    push({
      type: 'messageChunk',
      role: 'agent',
      text: 'Done with this turn. '.repeat(30),
    })
    push({ type: 'turnEnd', stop_reason: 'end_turn', error: null })
  }
  return events
}

/** The best of a few runs, so one slow pass (a GC, the JIT warming up) doesn't set the number. */
function time(label: string, run: () => void, runs = 5) {
  let best = Infinity
  for (let pass = 0; pass < runs; pass += 1) {
    const start = performance.now()
    run()
    best = Math.min(best, performance.now() - start)
  }
  console.log(`${label}: ${best.toFixed(1)}ms`)
  return best
}

describe('timeline pipeline at scale', () => {
  it('builds and streams a 5k-event thread quickly', () => {
    const events = syntheticThread(120)
    expect(events.length).toBeGreaterThan(5000)
    const builder = new TimelineBuilder()
    const input = (list: Timed[]) => ({
      events: list,
      approvals: [],
      findingsReady: false,
      showLive: true,
    })
    const first = time('first build (5k events)', () =>
      new TimelineBuilder().build(input(events)),
    )
    builder.build(input(events))
    // Streaming: 200 chunks appended to a reply at the end.
    let streamed = events
    const chunks = time('200 streamed chunks (rebuild each)', () => {
      streamed = events
      for (let i = 0; i < 200; i += 1) {
        streamed = [
          ...streamed,
          {
            seq: streamed.length + 1,
            atMs: 0,
            event: { type: 'messageChunk', role: 'agent', text: `chunk ${i} ` },
          },
        ]
        const items = builder.build(input(streamed))
        buildTimelineRows(items, 'Lead', 'Claude Code')
      }
    })
    const split = time('splitConversations (5k)', () =>
      splitConversations(events),
    )
    expect(first).toBeLessThan(5000)
    expect(chunks).toBeLessThan(10_000)
    expect(split).toBeLessThan(5000)
  })

  it('reuseRows on 200 unchanged rows', () => {
    const rows = Array.from({ length: 200 }, (_, i) => ({
      id: `w${i}`,
      name: `Workspace ${i}`,
      branch: `feat/${i}`,
      lifecycle: 'idle',
      activityText: 'Idle',
      pendingApprovals: 0,
      summary: 'x'.repeat(400),
    }))
    const copy = rows.map((row) => ({ ...row }))
    let out: unknown
    const ms = time('reuseRows 200 x 50', () => {
      for (let i = 0; i < 50; i += 1) out = reuseRows(rows, copy)
    })
    expect(out).toBe(rows)
    expect(ms).toBeLessThan(5000)
  })
})
