import type { ApprovalRow } from '$lib/ipc/bindings'
import { promptHistory } from '$lib/composer/prompt-history'
import { splitConversations, type Conversation } from './conversations'
import { activeIds, liveStatus, type LiveStatus } from './live-status'
import { hasSessionToClear } from './new-session'
import type { TimedEvent } from './timed-events'
import { TimelineBuilder } from './timeline/builder'
import type { TimelineItem } from './timeline-types'

/** Everything a thread's conversation shows, from its events and its row. */
export type ThreadView = {
  /** False until the thread's history has been read, so an empty thread can be told from a loading one. */
  loaded: boolean
  /** Earlier history is in the store than is loaded. */
  hasOlder: boolean
  /** The conversation showing now; earlier ones can be resumed. */
  items: TimelineItem[]
  live: LiveStatus | null
  conversations: { current: Conversation; earlier: Conversation[] }
  /** False when the agent has nothing to forget, so New session has nothing to do. */
  canClearSession: boolean
  /** Your prompts, oldest first, for ↑/↓ in the composer. */
  readonly promptHistory: string[]
  /** What the agent is still writing: its reply, thought or run, while it works. */
  activeIds: {
    reply: string | null
    thought: string | null
    run: string | null
  }
  /** Bumped when older rows load, so the rows they add don't enter as if new. */
  generation: number
}

/** The thread's row, as far as its conversation cares. */
export type ThreadMeta = {
  status: string
  paused: boolean
  activity: string
  lead: boolean
}

export type ViewSources = {
  /** Undefined until the thread's history has been read. */
  events: TimedEvent[] | undefined
  approvals: ApprovalRow[]
  meta: ThreadMeta | undefined
  currentTool: string | null
  hasOlder: boolean
  generation: number
}

const NO_EVENTS: TimedEvent[] = []
const conversationsByEvents = new WeakMap<
  TimedEvent[],
  ReturnType<typeof splitConversations>
>()
const promptsByEvents = new WeakMap<TimedEvent[], string[]>()

function conversationsOf(events: TimedEvent[]) {
  let split = conversationsByEvents.get(events)
  if (!split) {
    split = splitConversations(events)
    conversationsByEvents.set(events, split)
  }
  return split
}

function promptsOf(events: TimedEvent[]) {
  let prompts = promptsByEvents.get(events)
  if (!prompts) {
    prompts = promptHistory(events)
    promptsByEvents.set(events, prompts)
  }
  return prompts
}

function sameSources(a: ViewSources, b: ViewSources) {
  return (
    a.events === b.events &&
    a.approvals === b.approvals &&
    a.currentTool === b.currentTool &&
    a.hasOlder === b.hasOlder &&
    a.generation === b.generation &&
    a.meta?.status === b.meta?.status &&
    a.meta?.paused === b.meta?.paused &&
    a.meta?.activity === b.meta?.activity &&
    a.meta?.lead === b.meta?.lead
  )
}

/**
 * One thread's view, rebuilt only when what it's made from changes: asking again with the same
 * sources hands back the same object, so every reader of a thread shares one build.
 */
export class ThreadViewBuilder {
  private timeline = new TimelineBuilder()
  private last: { sources: ViewSources; view: ThreadView } | null = null

  view(sources: ViewSources): ThreadView {
    if (this.last && sameSources(this.last.sources, sources)) {
      return this.last.view
    }
    const events = sources.events ?? NO_EVENTS
    const conversations = conversationsOf(events)
    const fold = this.timeline.build({
      events: conversations.current.events,
      approvals: sources.approvals,
    })
    const meta = sources.meta
    const live = meta
      ? liveStatus({
          ...meta,
          currentTool: sources.currentTool,
          agents: fold.agents,
        })
      : null
    const previous = this.last?.view
    const view: ThreadView = {
      loaded: sources.events !== undefined,
      hasOlder: sources.hasOlder,
      items: fold.items,
      // The same object while it reads the same, so the status above the prompt doesn't redraw.
      live:
        previous?.live && live && sameLive(previous.live, live)
          ? previous.live
          : live,
      conversations,
      canClearSession: hasSessionToClear(events),
      get promptHistory() {
        return promptsOf(events)
      },
      activeIds: activeIds(fold.items, live !== null),
      generation: sources.generation,
    }
    this.last = { sources, view }
    return view
  }
}

function sameLive(a: LiveStatus, b: LiveStatus) {
  return (
    a.title === b.title &&
    a.subtitle === b.subtitle &&
    a.paused === b.paused &&
    a.agents === b.agents
  )
}
