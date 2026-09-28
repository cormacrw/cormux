import type {
  AgentEvent,
  ApprovalRow,
  SettingRow,
  ThreadEventRow,
} from '$lib/ipc/bindings'
import { agentEventsForThread } from '$lib/thread/parse-agent-event'
import {
  mapEventsToTimeline,
  shouldShowLiveRow,
} from '$lib/thread/map-events-to-timeline'
import type { TimelineItem } from '$lib/thread/timeline-types'
import { threads } from './threads.svelte'
import { reviewFindingsReady } from '$lib/thread/review-findings'

type ThreadMeta = {
  status: string
  paused: boolean
  role: string
  workspaceId: string
}

export class ThreadTimelineStore {
  eventsByThread = $state<
    Record<string, { seq: number; atMs: number; event: AgentEvent }[]>
  >({})
  currentToolByThread = $state<Record<string, string>>({})
  private nextSeqByThread = $state<Record<string, number>>({})
  private approvals = $state<ApprovalRow[]>([])
  private settings = $state<SettingRow[]>([])

  hydrate(input: {
    timeline: ThreadEventRow[]
    threadIds: string[]
    approvals: ApprovalRow[]
    settings: SettingRow[]
    threadMeta: Record<string, ThreadMeta>
    findingCountByWorkspace: Record<string, number>
  }) {
    const nextEvents: Record<
      string,
      { seq: number; atMs: number; event: AgentEvent }[]
    > = {}
    const nextSeq: Record<string, number> = {}

    for (const threadId of input.threadIds) {
      const events = agentEventsForThread(input.timeline, threadId)
      nextEvents[threadId] = events
      nextSeq[threadId] = events.at(-1)?.seq ?? 0
    }

    this.eventsByThread = nextEvents
    this.nextSeqByThread = nextSeq
    this.approvals = input.approvals
    this.settings = input.settings
    void input.threadMeta
    void input.findingCountByWorkspace
  }

  itemsForThread(
    threadId: string,
    meta: ThreadMeta,
    findingCount: number,
  ): TimelineItem[] {
    const events = this.eventsByThread[threadId] ?? []
    const threadApprovals = this.approvals.filter(
      (row) => row.threadId === threadId,
    )
    return mapEventsToTimeline({
      events,
      approvals: threadApprovals,
      findingsReady: reviewFindingsReady(
        this.settings,
        meta.workspaceId,
        meta.role,
        findingCount,
      ),
      showLive: shouldShowLiveRow(meta.status, meta.paused),
      liveToolTitle: this.currentToolByThread[threadId] ?? null,
    })
  }

  applyEvent(threadId: string, event: AgentEvent) {
    const seq = (this.nextSeqByThread[threadId] ?? 0) + 1
    this.nextSeqByThread = { ...this.nextSeqByThread, [threadId]: seq }
    const prev = this.eventsByThread[threadId] ?? []
    this.eventsByThread = {
      ...this.eventsByThread,
      [threadId]: [...prev, { seq, atMs: Date.now(), event }],
    }
    if (event.type === 'currentTool') {
      this.currentToolByThread = {
        ...this.currentToolByThread,
        [threadId]: event.title,
      }
    }
    if (event.type === 'turnEnd' || event.type === 'engineExited') {
      const next = { ...this.currentToolByThread }
      delete next[threadId]
      this.currentToolByThread = next
      threads.setStatus(threadId, 'idle')
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
