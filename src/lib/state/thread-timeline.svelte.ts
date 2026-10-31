// Bookkeeping that nothing renders; the reactive state is the $state fields.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type {
  AgentEvent,
  ApprovalRow,
  SettingRow,
  StreamedEvent,
  ThreadActivityRow,
  ThreadEventRow,
} from '$lib/ipc/bindings'
import { commands } from '$lib/ipc/bindings'
import { isThreadActivity } from '$lib/sidebar/thread-list'
import { createEventBatcher } from '$lib/thread/event-batcher'
import { threadsToEvict } from '$lib/thread/evict-threads'
import { eventCreatedAtMs } from '$lib/thread/thread-time'
import { parseAgentEventPayload } from '$lib/thread/parse-agent-event'
import {
  TimelineBuilder,
  shouldShowLiveRow,
} from '$lib/thread/map-events-to-timeline'
import {
  appendStreamed,
  mergeNewerRows,
  type TimedEvent,
} from '$lib/thread/reconcile-events'
import type { TimelineItem } from '$lib/thread/timeline-types'
import { bindVisibleThreadEvents } from '$lib/workspace/visible-thread-events'
import { threads } from './threads.svelte'
import { reviewFindingsReady } from '$lib/thread/review-findings'
import { splitConversations } from '$lib/thread/conversations'

type ThreadMeta = {
  status: string
  paused: boolean
  role: string
  workspaceId: string
  /** Scratch threads mark their reads `Read-only, no worktree`. */
  scratch?: boolean
}

const PAGE = 2000
// Threads kept in memory for an instant switch back, on top of the ones on screen.
const LOADED_THREADS = 8

type ThreadHistory = {
  /** The newest and oldest rows read from the store; events after `newestSeq` are local. */
  newestSeq: number
  oldestSeq: number
  refreshing: boolean
  refreshAgain: boolean
}

function timedEvents(rows: ThreadEventRow[]): TimedEvent[] {
  return rows.flatMap((row) => {
    const event = parseAgentEventPayload(row.payload, row.seq)
    return event
      ? [{ seq: row.seq, atMs: eventCreatedAtMs(row.createdAt), event }]
      : []
  })
}

export class ThreadTimelineStore {
  // Raw, not deep: a long thread holds thousands of events, and reading each through a
  // reactive proxy made every rebuild of the timeline ~30x slower. Always replaced, never mutated.
  // Only the threads you've opened recently are here; see `watch`.
  eventsByThread = $state.raw<Record<string, TimedEvent[]>>({})
  /** When each thread last moved, for the sidebar; streamed output doesn't count. */
  activityByThread = $state.raw<Record<string, number>>({})
  currentToolByThread = $state<Record<string, string>>({})
  /** Threads with earlier history in the store than is loaded. */
  olderByThread = $state.raw<Record<string, boolean>>({})
  /** Bumped when older rows load, since that renumbers the rows on screen. */
  historyGeneration = $state.raw<Record<string, number>>({})
  private approvals = $state.raw<ApprovalRow[]>([])
  private settings = $state.raw<SettingRow[]>([])

  private history = new Map<string, ThreadHistory>()
  private builders = new Map<string, TimelineBuilder>()
  private watched = new Map<string, number>()
  /** Oldest first. */
  private recent: string[] = []
  private conversationsCache = new WeakMap<
    TimedEvent[],
    ReturnType<typeof splitConversations>
  >()
  private batcher = createEventBatcher<{
    threadId: string
    streamed: StreamedEvent
  }>((batch) => {
    for (const threadId of new Set(batch.map((entry) => entry.threadId))) {
      this.applyEvents(
        threadId,
        batch.flatMap((entry) =>
          entry.threadId === threadId ? [entry.streamed] : [],
        ),
      )
    }
  })

  hydrate(input: {
    threadIds: string[]
    approvals: ApprovalRow[]
    settings: SettingRow[]
    activity: ThreadActivityRow[]
  }) {
    const alive = new Set(input.threadIds)
    for (const threadId of Object.keys(this.eventsByThread)) {
      if (!alive.has(threadId)) this.forget(threadId)
    }
    if (JSON.stringify(input.approvals) !== JSON.stringify(this.approvals)) {
      this.approvals = input.approvals
    }
    if (JSON.stringify(input.settings) !== JSON.stringify(this.settings)) {
      this.settings = input.settings
    }
    this.mergeActivity(input.activity)
    // Steps the core writes itself (a pause, a pull, a new session) aren't streamed.
    for (const threadId of this.watched.keys()) void this.refresh(threadId)
  }

  private mergeActivity(rows: ThreadActivityRow[]) {
    let next: Record<string, number> | null = null
    for (const row of rows) {
      const atMs = eventCreatedAtMs(row.createdAt)
      if (atMs > (this.activityByThread[row.threadId] ?? 0)) {
        next ??= { ...this.activityByThread }
        next[row.threadId] = atMs
      }
    }
    if (next) this.activityByThread = next
  }

  /** True once the thread's history has been read, so an empty thread can be told from a loading one. */
  isLoaded(threadId: string) {
    return this.eventsByThread[threadId] !== undefined
  }

  itemsForThread(
    threadId: string,
    meta: ThreadMeta,
    findingCount: number,
  ): TimelineItem[] {
    // Only the conversation showing now; earlier ones can be resumed.
    const events = this.conversations(threadId).current.events
    const threadApprovals = this.approvals.filter(
      (row) => row.threadId === threadId,
    )
    let builder = this.builders.get(threadId)
    if (!builder) {
      builder = new TimelineBuilder()
      this.builders.set(threadId, builder)
    }
    return builder.build({
      events,
      approvals: threadApprovals,
      findingsReady: reviewFindingsReady(
        this.settings,
        meta.workspaceId,
        findingCount,
      ),
      showLive: shouldShowLiveRow(meta.status, meta.paused),
      scratch: meta.scratch,
    })
  }

  /** The thread's current conversation and the earlier ones (see conversations.ts). */
  conversations(threadId: string) {
    const events = this.eventsByThread[threadId] ?? []
    let split = this.conversationsCache.get(events)
    if (!split) {
      split = splitConversations(events)
      this.conversationsCache.set(events, split)
    }
    return split
  }

  /**
   * Keeps a thread on screen current: streams its events, reads whatever it missed while it was
   * off screen, and holds it in memory until the returned function is called.
   */
  watch(threadId: string): () => void {
    this.watched.set(threadId, (this.watched.get(threadId) ?? 0) + 1)
    this.markRecent(threadId)
    const stop = bindVisibleThreadEvents(threadId, (streamed) => {
      if (streamed.event.type === 'streamLagged') void this.refresh(threadId)
      else this.enqueueEvent(threadId, streamed)
    })
    // After subscribing, so nothing lands between the read and the stream.
    void this.refresh(threadId)
    return () => {
      stop()
      this.batcher.flush()
      const count = (this.watched.get(threadId) ?? 1) - 1
      if (count > 0) this.watched.set(threadId, count)
      else this.watched.delete(threadId)
      this.evict()
    }
  }

  private markRecent(threadId: string) {
    this.recent = [...this.recent.filter((id) => id !== threadId), threadId]
    this.evict()
  }

  private evict() {
    const watched = new Set(this.watched.keys())
    for (const threadId of threadsToEvict(
      this.recent,
      watched,
      LOADED_THREADS,
    )) {
      this.forget(threadId)
    }
  }

  private forget(threadId: string) {
    this.recent = this.recent.filter((id) => id !== threadId)
    this.history.delete(threadId)
    this.builders.delete(threadId)
    const events = { ...this.eventsByThread }
    delete events[threadId]
    this.eventsByThread = events
    const older = { ...this.olderByThread }
    delete older[threadId]
    this.olderByThread = older
  }

  /** Brings the thread up to the store's latest, keeping anything shown that isn't saved yet. */
  async refresh(threadId: string): Promise<void> {
    const known = this.history.get(threadId)
    if (known?.refreshing) {
      known.refreshAgain = true
      return
    }
    const history = known ?? {
      newestSeq: 0,
      oldestSeq: 0,
      refreshing: false,
      refreshAgain: false,
    }
    this.history.set(threadId, history)
    history.refreshing = true
    try {
      do {
        history.refreshAgain = false
        if (history.newestSeq === 0) {
          await this.loadNewest(threadId, history)
        } else {
          await this.loadNewer(threadId, history)
        }
      } while (history.refreshAgain)
    } finally {
      history.refreshing = false
    }
  }

  private async loadNewest(threadId: string, history: ThreadHistory) {
    const result = await commands.getThreadEvents(threadId, null, null, PAGE)
    if (result.status === 'error' || this.history.get(threadId) !== history) {
      return
    }
    const rows = result.data.events
    const newest = rows.at(-1)?.seq ?? 0
    history.newestSeq = newest
    history.oldestSeq = rows[0]?.seq ?? 0
    // Nothing shown yet came from the store, so all of it is laid over this read.
    this.eventsByThread = {
      ...this.eventsByThread,
      [threadId]: mergeNewerRows(
        this.eventsByThread[threadId] ?? [],
        0,
        timedEvents(rows),
        newest,
        Date.now(),
      ),
    }
    if (result.data.hasOlder) {
      this.olderByThread = { ...this.olderByThread, [threadId]: true }
    }
  }

  private async loadNewer(threadId: string, history: ThreadHistory) {
    const fetched: ThreadEventRow[] = []
    for (;;) {
      const result = await commands.getThreadEvents(
        threadId,
        fetched.at(-1)?.seq ?? history.newestSeq,
        null,
        PAGE,
      )
      if (result.status === 'error' || this.history.get(threadId) !== history) {
        return
      }
      fetched.push(...result.data.events)
      if (result.data.events.length < PAGE) break
    }
    const newest = fetched.at(-1)?.seq ?? history.newestSeq
    const shown = this.eventsByThread[threadId] ?? []
    const merged = mergeNewerRows(
      shown,
      history.newestSeq,
      timedEvents(fetched),
      newest,
      Date.now(),
    )
    history.newestSeq = newest
    if (merged !== shown) {
      this.eventsByThread = { ...this.eventsByThread, [threadId]: merged }
    }
  }

  /** Reads the page before what's loaded. Resolves false if nothing more was added. */
  async loadOlder(threadId: string): Promise<boolean> {
    const history = this.history.get(threadId)
    if (!history || !this.olderByThread[threadId]) return false
    const result = await commands.getThreadEvents(
      threadId,
      null,
      history.oldestSeq,
      PAGE,
    )
    if (result.status === 'error' || this.history.get(threadId) !== history) {
      return false
    }
    const rows = result.data.events
    history.oldestSeq = rows[0]?.seq ?? history.oldestSeq
    this.olderByThread = {
      ...this.olderByThread,
      [threadId]: result.data.hasOlder,
    }
    this.historyGeneration = {
      ...this.historyGeneration,
      [threadId]: (this.historyGeneration[threadId] ?? 0) + 1,
    }
    this.eventsByThread = {
      ...this.eventsByThread,
      [threadId]: [
        ...timedEvents(rows),
        ...(this.eventsByThread[threadId] ?? []),
      ],
    }
    return rows.length > 0
  }

  /** For streamed events: they land together, once per frame. */
  enqueueEvent(threadId: string, streamed: StreamedEvent) {
    this.batcher.push({ threadId, streamed })
  }

  /** Shows an event now, after anything still queued. */
  applyEvent(threadId: string, event: AgentEvent) {
    this.batcher.flush()
    this.applyEvents(threadId, [{ seq: null, event }])
  }

  private applyEvents(threadId: string, batch: StreamedEvent[]) {
    const prev = this.eventsByThread[threadId] ?? []
    const atMs = Date.now()
    const shown = appendStreamed(
      prev,
      this.history.get(threadId)?.newestSeq ?? 0,
      batch,
      atMs,
    )
    if (shown !== prev) {
      this.eventsByThread = { ...this.eventsByThread, [threadId]: shown }
    }
    const events = batch.map((streamed) => streamed.event)
    if (events.some(isThreadActivity)) {
      this.activityByThread = { ...this.activityByThread, [threadId]: atMs }
    }
    let liveTool: string | null = null
    let ended = false
    for (const event of events) {
      if (event.type === 'currentTool' || event.type === 'toolCallUpdate') {
        liveTool = event.title ?? liveTool
      }
      if (event.type === 'turnEnd' || event.type === 'engineExited') {
        ended = true
        liveTool = null
      }
    }
    if (liveTool) {
      this.currentToolByThread = {
        ...this.currentToolByThread,
        [threadId]: liveTool,
      }
    }
    if (ended) {
      if (!liveTool) {
        const next = { ...this.currentToolByThread }
        delete next[threadId]
        this.currentToolByThread = next
      }
      threads.setStatus(threadId, 'idle')
      // Trades the local sequence numbers for the store's, and picks up what wasn't streamed.
      if (this.watched.has(threadId)) void this.refresh(threadId)
    }
  }

  appendStreamChunk(
    threadId: string,
    text: string,
    role: 'user' | 'agent' | 'thought' = 'agent',
  ) {
    this.applyEvent(threadId, {
      type: 'messageChunk',
      role,
      text,
    })
  }
}

export const threadTimeline = new ThreadTimelineStore()
