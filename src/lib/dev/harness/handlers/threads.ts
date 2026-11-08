import type { PromptAttachment, TranscriptEvent } from '$lib/ipc/bindings'
import { fixtureSnapshot, fixtureTimeline } from '../../fixture-snapshot'
import type { HarnessRuntime } from '../runtime'
import { nextSeq, shownMessage } from '../seed'
import type { HandlerSlice, Input } from '../types'

const MODELS = [
  { id: 'fable', label: 'Fable' },
  { id: 'opus', label: 'Opus' },
  { id: 'sonnet', label: 'Sonnet' },
  { id: 'haiku', label: 'Haiku' },
]

const EFFORTS = [
  { id: 'low', label: 'Low' },
  { id: 'medium', label: 'Medium' },
  { id: 'high', label: 'High' },
  { id: 'xhigh', label: 'Extra high' },
  { id: 'max', label: 'Max' },
]

const PIXEL_PNG =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8Xw8AAwMCAO+ip1sAAAAASUVORK5CYII='

function findThread(threadId: string) {
  return fixtureSnapshot.persisted.threads.find((row) => row.id === threadId)
}

/** Matches `thread_event_page` in the core's store. */
function threadEventPage(args: {
  threadId: string
  afterSeq: number | null
  beforeSeq: number | null
  limit: number | null
}) {
  const rows = fixtureTimeline
    .filter((row) => row.threadId === args.threadId)
    .sort((a, b) => a.seq - b.seq)
  const limit = args.limit ?? 2000
  const after = args.afterSeq
  const before = args.beforeSeq
  const events =
    after != null
      ? rows.filter((row) => row.seq > after).slice(0, limit)
      : rows.filter((row) => before == null || row.seq < before).slice(-limit)
  const first = events[0]
  return {
    events,
    hasOlder: first ? rows.some((row) => row.seq < first.seq) : false,
  }
}

export function threadHandlers(runtime: HarnessRuntime) {
  const {
    scenario,
    controls,
    agentChannels,
    threadModels,
    threadEfforts,
    emitStateChanged,
  } = runtime
  // Stops the reply running in a thread, as interrupting the engine would.
  const stopReply = new Map<string, () => void>()

  /** Saves the event like the core's transcript, then streams it with its `seq`. */
  const append = (threadId: string, kind: string, event: TranscriptEvent) => {
    const seq = nextSeq(threadId)
    fixtureTimeline.push({
      id: 900_000 + seq,
      threadId,
      seq,
      kind,
      payload: JSON.stringify(event),
      createdAt: new Date().toISOString().replace('T', ' ').replace('Z', ''),
    })
    const channel = agentChannels.get(threadId)
    if (channel) runtime.send(channel, { seq, event })
    return seq
  }

  // Stream a reply the way a real engine does: many small chunks, then a turn end that
  // the backend follows with a state-changed refetch.
  const streamReply = (threadId: string, text: string) => {
    let stopped = false
    stopReply.set(threadId, () => {
      stopped = true
    })
    // One tool call first, the way agents read before they answer.
    setTimeout(() => {
      if (stopped) return
      append(threadId, 'tool', {
        type: 'toolCall',
        id: `harness-read-${nextSeq(threadId)}`,
        title: 'Read',
        role: { kind: 'read' },
        status: 'completed',
        locations: ['src/lib/auth.ts'],
        detail: 'src/lib/auth.ts',
      })
    }, 40)
    const slow = scenario.slowReply === true
    const wordMs = slow ? 40 : 10
    const body = slow
      ? Array.from(
          { length: 12 },
          (_, n) => `Paragraph ${n + 1}: ${'streamed word '.repeat(25)}`,
        ).join('\n\n')
      : 'streamed word '.repeat(40)
    const words = `Reply to "${text}": ${body}done.`.split(' ')
    words.forEach((word, i) => {
      setTimeout(
        () => {
          if (stopped) return
          append(threadId, 'message', {
            type: 'messageChunk',
            role: 'agent',
            text: `${i ? ' ' : ''}${word}`,
          })
        },
        50 + i * wordMs,
      )
    })
    setTimeout(
      () => {
        if (stopped) return
        stopReply.delete(threadId)
        append(threadId, 'turn_end', {
          type: 'turnEnd',
          stop_reason: 'end_turn',
          error: null,
        })
        const lead = findThread(threadId)
        if (lead) lead.status = 'idle'
        emitStateChanged()
      },
      80 + words.length * wordMs,
    )
  }

  /** Ends the thread's session, then notes that in its timeline like the core does. */
  const switchSession = (
    threadId: string,
    resume: Input<'resume_thread_session'>['input'] | null,
  ) => {
    const thread = findThread(threadId)
    const ended = thread?.sessionId ?? null
    append(threadId, 'tool', {
      type: 'step',
      id: `step-${nextSeq(threadId)}`,
      title: resume
        ? 'Resumed an earlier conversation'
        : 'Started a new session',
      detail: null,
      step: resume
        ? {
            kind: 'sessionResumed',
            conversation: resume.conversationId,
            ended_session: ended,
          }
        : { kind: 'sessionStarted', ended_session: ended },
    })
    if (thread) {
      thread.status = 'idle'
      thread.sessionId = resume ? resume.sessionId : null
    }
    emitStateChanged()
    return null
  }

  return {
    get_thread_events: threadEventPage,
    subscribe_agent_events: ({
      threadId,
      channel,
    }: {
      threadId: string
      channel: { id: number }
    }) => {
      agentChannels.set(threadId, { id: channel.id, index: 0 })
      return runtime.newId()
    },
    send_thread_prompt: (args: {
      threadId: string
      text: string
      attachments: PromptAttachment[]
    }) => {
      controls.prompts.push(args)
      const text = shownMessage(
        args.text.trim(),
        args.attachments.map((attachment) => attachment.name),
      )
      if (!text) return null
      const seq = append(args.threadId, 'message', {
        type: 'messageChunk',
        role: 'user',
        text,
      })
      const thread = findThread(args.threadId)
      if (thread) thread.status = 'running'
      emitStateChanged()
      if (args.threadId === 'th-lead') streamReply('th-lead', args.text)
      return { seq, text }
    },
    cancel_thread_turn: ({ threadId }: { threadId: string }) => {
      stopReply.get(threadId)?.()
      stopReply.delete(threadId)
      append(threadId, 'turn_end', {
        type: 'turnEnd',
        stop_reason: 'cancelled',
        error: null,
      })
      append(threadId, 'tool', {
        type: 'step',
        id: `step-${nextSeq(threadId)}`,
        title: 'You stopped the agent',
        detail: null,
        step: { kind: 'agentStopped' },
      })
      const thread = findThread(threadId)
      if (thread) thread.status = 'idle'
      emitStateChanged()
      return null
    },
    new_thread_session: ({ threadId }: { threadId: string }) =>
      switchSession(threadId, null),
    resume_thread_session: ({ input }: Input<'resume_thread_session'>) => {
      controls.resumed = input
      return switchSession(input.threadId, input)
    },
    engine_models: () => MODELS,
    thread_models: ({ threadId }: { threadId: string }) => ({
      current: threadModels.get(threadId) ?? null,
      options: MODELS,
    }),
    set_thread_model: ({
      threadId,
      model,
    }: {
      threadId: string
      model: string | null
    }) => {
      threadModels.set(threadId, model ?? null)
      return null
    },
    engine_efforts: () => EFFORTS,
    thread_efforts: ({ threadId }: { threadId: string }) => ({
      current: threadEfforts.get(threadId) ?? null,
      options: EFFORTS,
    }),
    set_thread_effort: ({
      threadId,
      effort,
    }: {
      threadId: string
      effort: string | null
    }) => {
      threadEfforts.set(threadId, effort ?? null)
      return null
    },
    list_thread_skills: () => [
      {
        name: 'deploy',
        description: 'Build and ship the app to staging',
        source: 'project',
      },
      {
        name: 'code-review',
        description: 'Review the current diff for bugs',
        source: 'user',
      },
      {
        name: 'release-notes',
        description: 'Draft release notes from merged PRs',
        source: 'user',
      },
    ],
    answer_question: ({
      id,
      answers,
    }: {
      id: string
      answers: Record<string, string>
    }) => {
      controls.answered.push({ id, answers })
      const row = fixtureSnapshot.persisted.approvals.find(
        (entry) => entry.id === id,
      )
      if (row?.payload) {
        row.status = 'approved'
        row.payload = { ...row.payload, resolvedAtMs: Date.now(), answers }
      }
      emitStateChanged()
      return null
    },
    resolve_approval: ({
      id,
      approved,
      denyReason,
    }: {
      id: string
      approved: boolean
      denyReason: string | null
    }) => {
      const reason = denyReason ?? null
      controls.resolved.push({ id, approved, denyReason: reason })
      const row = fixtureSnapshot.persisted.approvals.find(
        (entry) => entry.id === id,
      )
      if (row) {
        row.status = approved ? 'approved' : 'denied'
        if (row.payload) {
          row.payload = {
            ...row.payload,
            resolvedAtMs: Date.now(),
            ...(reason ? { denyReason: reason } : {}),
          }
        }
      }
      emitStateChanged()
      return null
    },
    resolve_thread_media: ({ source }: { source: string }) => {
      const name = source.split('/').filter(Boolean).pop() ?? source
      const kind = /\.(mp4|webm|mov|m4v)$/i.test(name) ? 'video' : 'image'
      return {
        kind,
        name,
        path: `/fixtures/${name}`,
        previewUrl: kind === 'image' ? PIXEL_PNG : null,
      }
    },
    // The core returns null for a file it can't decode, and the chip then goes without a thumbnail.
    image_preview: () => null,
  } satisfies HandlerSlice
}
