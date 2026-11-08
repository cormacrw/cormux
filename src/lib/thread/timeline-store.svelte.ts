// Bookkeeping that nothing renders; the reactive state is the $state fields.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type {
  ApprovalRow,
  StreamedEvent,
  ThreadActivityRow,
  ThreadEventRow,
} from '$lib/ipc/bindings'
import { commands } from '$lib/ipc/bindings'
import type { CoreError } from '$lib/ipc'
import { subscribeAgentEvents } from '$lib/ipc/channels'
import {
  toPromptAttachment,
  type ComposerAttachment,
} from '$lib/composer/attachments'
import { deepEqual } from '$lib/deep-equal'
import { onSnapshot } from '$lib/core/sync'
import type { Conversation } from '$lib/thread/conversations'
import { createEventBatcher } from '$lib/thread/event-batcher'
import { threadsToEvict } from '$lib/thread/evict-threads'
import { parseAgentEventPayload } from '$lib/thread/parse-agent-event'
import { SharedSubscriptions } from '$lib/thread/shared-subscriptions'
import { eventCreatedAtMs } from '$lib/thread/thread-time'
import { ThreadViewBuilder, type ThreadView } from '$lib/thread/thread-view'
import {
  appendStreamed,
  mergeSaved,
  type TimedEvent,
} from '$lib/thread/timed-events'
import { threads } from '../state/threads.svelte'

const PAGE = 2000
// Threads kept in memory for an instant switch back, on top of the ones on screen.
const LOADED_THREADS = 8
const NO_APPROVALS: ApprovalRow[] = []

type ThreadHistory = {
  /** The newest and oldest rows read from the store; streamed ones can be newer. */
  newestSeq: number
  oldestSeq: number
  refreshing: boolean
  refreshAgain: boolean
}

function timedEvents(rows: ThreadEventRow[]): TimedEvent[] {
  return rows.flatMap((row) => {
    const event = parseAgentEventPayload(row.payload)
    return event
      ? [{ seq: row.seq, atMs: eventCreatedAtMs(row.createdAt), event }]
      : []
  })
}

/** Why an action failed: the core's error, or what threw, with what was being tried. */
export type ThreadActionError = { error: CoreError | unknown; doing: string }

/** Owns every thread's loaded history, its stream, and the view and actions built on them. */
export class ThreadTimelineStore {
  // Raw, not deep: a long thread holds thousands of events, and reading each through a
  // reactive proxy made every rebuild of the timeline ~30x slower. Always replaced, never mutated.
  // Only the threads you've opened recently are here; see `watch`.
  private eventsByThread = $state.raw<Record<string, TimedEvent[]>>({})
  /** When each thread last moved, for the sidebar; streamed output doesn't count. */
  activityByThread = $state.raw<Record<string, number>>({})
  private currentToolByThread = $state.raw<Record<string, string>>({})
  private olderByThread = $state.raw<Record<string, boolean>>({})
  private historyGeneration = $state.raw<Record<string, number>>({})
  private approvals = $state.raw<ApprovalRow[]>([])

  private history = new Map<string, ThreadHistory>()
  private views = new Map<string, ThreadViewBuilder>()
  private streams = new SharedSubscriptions((threadId) =>
    subscribeAgentEvents(threadId, (streamed) => {
      if (streamed.event.type === 'streamLagged') void this.refresh(threadId)
      else this.enqueueEvent(threadId, streamed)
    }),
  )
  /** Oldest first. */
  private recent: string[] = []
  private userMessageWaiters = new Map<string, () => void>()
  private readErrorListeners = new Set<(error: CoreError) => void>()
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

  /** Hears a background read of a thread fail; nothing else asked for it, so nothing else would. */
  onReadError(listener: (error: CoreError) => void) {
    this.readErrorListeners.add(listener)
    return () => void this.readErrorListeners.delete(listener)
  }

  private readFailed(error: CoreError) {
    for (const listener of this.readErrorListeners) listener(error)
  }

  hydrate(input: {
    threadIds: string[]
    approvals: ApprovalRow[]
    activity: ThreadActivityRow[]
  }) {
    const alive = new Set(input.threadIds)
    for (const threadId of Object.keys(this.eventsByThread)) {
      if (!alive.has(threadId)) this.forget(threadId)
    }
    if (!deepEqual(input.approvals, this.approvals)) {
      this.approvals = input.approvals
    }
    this.mergeActivity(input.activity)
  }

  // Grouped once per approvals list, so a timeline rebuild on every chunk doesn't scan them all.
  private approvalsByThread = $derived.by(() => {
    const map = new Map<string, ApprovalRow[]>()
    for (const row of this.approvals) {
      const list = map.get(row.threadId)
      if (list) list.push(row)
      else map.set(row.threadId, [row])
    }
    return map
  })

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

  /** Everything the thread's conversation shows; the same object until something in it changes. */
  threadView(threadId: string): ThreadView {
    let builder = this.views.get(threadId)
    if (!builder) {
      builder = new ThreadViewBuilder()
      this.views.set(threadId, builder)
    }
    // A scratch has no thread row: it can't run any more, so it has nothing live to show.
    const thread = threads.getById(threadId)
    return builder.view({
      events: this.eventsByThread[threadId],
      approvals: this.approvalsByThread.get(threadId) ?? NO_APPROVALS,
      meta: thread && {
        status: thread.status,
        paused: thread.status === 'paused',
        activity: thread.activity,
        lead: thread.ownRole === 'lead',
      },
      currentTool: this.currentToolByThread[threadId] ?? null,
      hasOlder: this.olderByThread[threadId] ?? false,
      generation: this.historyGeneration[threadId] ?? 0,
    })
  }

  /**
   * Keeps a thread on screen current: streams its events, reads whatever it missed while it was
   * off screen, and holds it in memory until the returned function is called.
   */
  watch(threadId: string): () => void {
    const stop = this.streams.watch(threadId)
    this.markRecent(threadId)
    // After subscribing, so nothing lands between the read and the stream.
    void this.refresh(threadId)
    return () => {
      stop()
      this.batcher.flush()
      this.evict()
    }
  }

  private markRecent(threadId: string) {
    this.recent = [...this.recent.filter((id) => id !== threadId), threadId]
    this.evict()
  }

  private evict() {
    for (const threadId of threadsToEvict(
      this.recent,
      this.streams.watched(),
      LOADED_THREADS,
    )) {
      this.forget(threadId)
    }
  }

  private forget(threadId: string) {
    this.recent = this.recent.filter((id) => id !== threadId)
    this.history.delete(threadId)
    this.views.delete(threadId)
    const events = { ...this.eventsByThread }
    delete events[threadId]
    this.eventsByThread = events
    const older = { ...this.olderByThread }
    delete older[threadId]
    this.olderByThread = older
  }

  /** Brings the thread up to the store's latest. */
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
    if (this.history.get(threadId) !== history) return
    if (result.status === 'error') {
      this.readFailed(result.error)
      return
    }
    const rows = result.data.events
    const newest = rows.at(-1)?.seq ?? 0
    history.newestSeq = newest
    history.oldestSeq = rows[0]?.seq ?? 0
    this.eventsByThread = {
      ...this.eventsByThread,
      [threadId]: mergeSaved(
        this.eventsByThread[threadId] ?? [],
        timedEvents(rows),
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
      if (this.history.get(threadId) !== history) return
      if (result.status === 'error') {
        this.readFailed(result.error)
        return
      }
      fetched.push(...result.data.events)
      if (result.data.events.length < PAGE) break
    }
    history.newestSeq = fetched.at(-1)?.seq ?? history.newestSeq
    const shown = this.eventsByThread[threadId] ?? []
    const merged = mergeSaved(shown, timedEvents(fetched))
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
    if (this.history.get(threadId) !== history) return false
    if (result.status === 'error') {
      this.readFailed(result.error)
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
      [threadId]: mergeSaved(
        this.eventsByThread[threadId] ?? [],
        timedEvents(rows),
      ),
    }
    return rows.length > 0
  }

  /**
   * Posts a message to the thread's agent. The core shows it in the thread and moves the
   * thread's status; resolves why it didn't send, or null.
   */
  async send(
    threadId: string,
    text: string,
    attachments: ComposerAttachment[] = [],
  ): Promise<ThreadActionError | null> {
    const doing = 'Could not reach the agent'
    try {
      const result = await commands.sendThreadPrompt(
        threadId,
        text,
        attachments.map(toPromptAttachment),
      )
      return result.status === 'ok' ? null : { error: result.error, doing }
    } catch (error) {
      return { error, doing }
    }
  }

  async stop(threadId: string): Promise<ThreadActionError | null> {
    const result = await commands.cancelThreadTurn(threadId)
    return result.status === 'error'
      ? { error: result.error, doing: 'Could not stop the agent' }
      : null
  }

  /**
   * Swaps the conversation for a fresh session, or back to an earlier one, whose session the
   * agent resumes.
   */
  async switchConversation(
    threadId: string,
    to: Conversation | 'new',
  ): Promise<ThreadActionError | null> {
    const result =
      to === 'new'
        ? await commands.newThreadSession(threadId)
        : await commands.resumeThreadSession({
            threadId,
            conversationId: to.id,
            sessionId: to.sessionId,
          })
    if (result.status === 'ok') return null
    return {
      error: result.error,
      doing:
        to === 'new'
          ? 'Could not start a new session'
          : 'Could not resume that conversation',
    }
  }

  /** For streamed events: they land together, once per frame, except your own messages. */
  private enqueueEvent(threadId: string, streamed: StreamedEvent) {
    this.batcher.push({ threadId, streamed })
    if (isUserMessage(streamed)) this.batcher.flush()
  }

  /** Resolves once a message from you lands in the thread, or after `ms`. */
  userMessageShown(threadId: string, ms = 1000): Promise<void> {
    return new Promise((resolve) => {
      const done = () => {
        clearTimeout(timer)
        if (this.userMessageWaiters.get(threadId) === done) {
          this.userMessageWaiters.delete(threadId)
        }
        resolve()
      }
      const timer = setTimeout(done, ms)
      this.userMessageWaiters.set(threadId, done)
    })
  }

  private applyEvents(threadId: string, batch: StreamedEvent[]) {
    const prev = this.eventsByThread[threadId] ?? []
    const shown = appendStreamed(prev, batch, Date.now())
    if (shown !== prev) {
      this.eventsByThread = { ...this.eventsByThread, [threadId]: shown }
    }
    if (batch.some(isUserMessage)) this.userMessageWaiters.get(threadId)?.()
    let liveTool: string | null = null
    let ended = false
    for (const { event } of batch) {
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
    if (ended && !liveTool) {
      const next = { ...this.currentToolByThread }
      delete next[threadId]
      this.currentToolByThread = next
    }
  }
}

function isUserMessage({ event }: StreamedEvent) {
  return event.type === 'messageChunk' && event.role === 'user'
}

export const threadTimeline = new ThreadTimelineStore()

onSnapshot((snapshot) =>
  threadTimeline.hydrate({
    threadIds: [
      ...snapshot.persisted.threads.map((thread) => thread.id),
      ...snapshot.persisted.scratches.map((scratch) => scratch.threadId),
    ],
    approvals: snapshot.persisted.approvals,
    activity: snapshot.persisted.threadActivity,
  }),
)
