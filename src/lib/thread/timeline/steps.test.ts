import { describe, expect, it } from 'vitest'
import type { TimelineStep, TranscriptEvent } from '$lib/ipc/bindings'
import { TimelineBuilder } from './builder'
import { foldTimeline } from './fold'

function render(...events: TranscriptEvent[]) {
  return foldTimeline({
    events: events.map((event, index) => ({
      seq: index + 1,
      atMs: 1000,
      event,
    })),
    approvals: [],
  }).items
}

const typed = (
  step: TimelineStep,
  title: string,
  detail: string | null = null,
  id = `step-${step.kind}`,
): TranscriptEvent => ({ type: 'step', id, title, detail, step })

const toolCall = (id: string, title: string): TranscriptEvent => ({
  type: 'toolCall',
  id,
  title,
  role: { kind: 'other', name: title },
  status: 'completed',
  locations: [],
  detail: null,
})

const prOpened: TimelineStep = {
  kind: 'prOpened',
  number: 12,
  branch: 'b',
  base: 'm',
}

describe('core steps in the timeline', () => {
  it('sit between turns, with their words and kind', () => {
    expect(render(typed(prOpened, 'Opened PR #12', 'b → m'))).toEqual([
      {
        kind: 'event',
        id: 'step-prOpened',
        step: prOpened,
        title: 'Opened PR #12',
        detail: 'b → m',
        seq: 1,
        atMs: 1000,
      },
    ])
  })

  it('join the tool steps around them when they run with them', () => {
    const items = render(
      toolCall('toolu_1', 'Thinking it over'),
      typed(
        { kind: 'findingsReceived', count: 2, titles: ['A', 'B'] },
        'Got them',
        'A · B',
      ),
    )
    expect(items).toMatchObject([
      {
        kind: 'toolRun',
        steps: [
          { kind: 'tool', id: 'toolu_1' },
          { kind: 'step', title: 'Got them', detail: 'A · B' },
        ],
      },
    ])
  })

  it('keep the first of a step written twice, as tool calls did', () => {
    const step = typed(prOpened, 'Opened PR #12', null, 'pr-open-12')
    expect(render(step, step)).toHaveLength(1)
  })

  it('treat a step repeating an id from a saved tool call as that call', () => {
    expect(
      render(
        toolCall('pr-open-12', 'Opened PR #12'),
        typed(prOpened, 'Opened PR #12', null, 'pr-open-12'),
      ),
    ).toHaveLength(1)
  })

  it('still show a kind this build does not know, between turns', () => {
    const event = {
      type: 'step',
      id: 'step-new',
      title: 'From the future',
      detail: null,
      step: { kind: 'somethingNew' },
    } as unknown as TranscriptEvent
    expect(render(event)).toMatchObject([
      { kind: 'event', title: 'From the future' },
    ])
  })

  it('saved by title by an older build go where they ran', () => {
    const legacy = (run: boolean) =>
      typed(
        { kind: 'legacy', title: 'Sent to the branch', icon: 'branch', run },
        'Sent to the branch',
      )
    expect(render(legacy(false))).toMatchObject([{ kind: 'event' }])
    expect(render(legacy(true))).toMatchObject([
      { kind: 'toolRun', steps: [{ kind: 'step' }] },
    ])
  })

  it('rebuild the same way incrementally', () => {
    const turnEnd: TranscriptEvent = {
      type: 'turnEnd',
      stop_reason: 'end_turn',
      error: null,
    }
    const step = typed(prOpened, 'PR is up', null, 'pr-open-12')
    const rows = [step, turnEnd, step].map((event, index) => ({
      seq: index + 1,
      atMs: 0,
      event,
    }))
    const input = { approvals: [] }
    const builder = new TimelineBuilder()
    builder.build({ ...input, events: rows.slice(0, 2) })
    expect(builder.build({ ...input, events: rows })).toEqual(
      foldTimeline({ ...input, events: rows }),
    )
  })
})
