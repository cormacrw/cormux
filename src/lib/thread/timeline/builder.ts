import type { ApprovalRow, TranscriptEvent } from '$lib/ipc/bindings'
import { deepEqual } from '$lib/deep-equal'
import type { RunningAgent, TimelineItem, ToolStep } from '../timeline-types'
import { sameApprovals } from './approval'
import {
  finishFold,
  foldEvents,
  foldState,
  type FoldInput,
  type TimelineFold,
} from './fold'

type FinishedTurns = {
  /** How many of the input's events these items cover. */
  count: number
  first: FoldInput['events'][number] | undefined
  last: FoldInput['events'][number] | undefined
  items: TimelineItem[]
  autoApproved: boolean
  callIds: Set<string>
  agents: ToolStep[]
  approvals: ApprovalRow[]
}

function touchesFinishedTurn(event: TranscriptEvent, callIds: Set<string>) {
  if (event.type === 'toolCallUpdate') return callIds.has(event.id)
  if (event.type === 'step') return callIds.has(event.id)
  if (event.type !== 'toolCall') return false
  return (
    callIds.has(event.id) ||
    (event.parent_id != null && callIds.has(event.parent_id))
  )
}

/**
 * The same timeline as `foldTimeline`, without redoing finished turns on every event, and
 * handing back the previous object for anything unchanged so its row isn't invalidated. Any event
 * that reaches back into a finished turn, or a change to the approvals, rebuilds from the start.
 */
export class TimelineBuilder {
  private finished: FinishedTurns | null = null
  private previous: TimelineItem[] = []
  private agents: RunningAgent[] = []

  private usableTurns(input: FoldInput): FinishedTurns | null {
    const turns = this.finished
    if (
      !turns ||
      input.events.length < turns.count ||
      input.events[0] !== turns.first ||
      input.events[turns.count - 1] !== turns.last ||
      !sameApprovals(turns.approvals, input.approvals)
    ) {
      return null
    }
    for (let index = turns.count; index < input.events.length; index += 1) {
      if (touchesFinishedTurn(input.events[index]!.event, turns.callIds)) {
        return null
      }
    }
    return turns
  }

  private fold(input: FoldInput): TimelineFold {
    const turns = this.usableTurns(input)
    if (!turns) this.finished = null
    const start = turns?.count ?? 0
    // Sealed: nothing written after a turn ends can extend a message from before it.
    const state = turns
      ? foldState([...turns.items], turns.autoApproved, true, [...turns.agents])
      : foldState()
    const tail = input.events.slice(start)
    const turnEnd = tail.findLastIndex(
      ({ event }) => event.type === 'turnEnd' || event.type === 'engineExited',
    )
    const boundary = tail[turnEnd]
    foldEvents(state, tail, input.approvals, (entry) => {
      if (entry !== boundary) return
      const callIds = new Set(turns?.callIds)
      for (const { event } of tail.slice(0, turnEnd + 1)) {
        if (event.type === 'toolCall' || event.type === 'step') {
          callIds.add(event.id)
        }
      }
      this.finished = {
        count: start + turnEnd + 1,
        first: input.events[0],
        last: boundary,
        items: [...state.items],
        autoApproved: state.autoApproved,
        callIds,
        agents: [...state.agents],
        approvals: input.approvals,
      }
    })
    return finishFold(state, input)
  }

  build(input: FoldInput): TimelineFold {
    const before = new Map(this.previous.map((item) => [item.id, item]))
    const reused = new Map<TimelineItem, TimelineItem>()
    const fold = this.fold(input)
    const items = fold.items.map((item) => {
      const old = before.get(item.id)
      if (!old || old === item) return item
      if (!deepEqual(old, item)) return item
      reused.set(item, old)
      return old
    })
    if (reused.size && this.finished) {
      this.finished.items = this.finished.items.map(
        (item) => reused.get(item) ?? item,
      )
    }
    const unchanged =
      items.length === this.previous.length &&
      items.every((item, index) => item === this.previous[index])
    if (!unchanged) this.previous = items
    if (!deepEqual(this.agents, fold.agents)) this.agents = fold.agents
    return { items: this.previous, agents: this.agents }
  }
}
