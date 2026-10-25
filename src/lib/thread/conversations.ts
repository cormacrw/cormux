import type { AgentEvent } from '$lib/ipc/bindings'
import { NEW_SESSION_TITLE } from './new-session'

/** Written by the core's `resume_thread_session`. */
export const RESUME_TITLE = 'Resumed an earlier conversation'

type TimedEvent = { seq: number; atMs: number; event: AgentEvent }

/**
 * One conversation in a thread: the events between session boundaries that
 * belong together. Starting a new session begins one; resuming switches back
 * to an earlier one, whose later events join it.
 */
export type Conversation = {
  /** `first` for the thread's opening conversation, else `after-<seq>` of the step that began it. */
  id: string
  events: TimedEvent[]
  /** The first thing you asked, to name it by. */
  firstPrompt: string | null
  prompts: number
  lastAtMs: number
  /** The agent session it ended with, to resume; null if never recorded. */
  sessionId: string | null
}

function stepDetail(event: AgentEvent): Record<string, unknown> | null {
  if (event.type !== 'toolCall' || !event.detail) return null
  try {
    const parsed: unknown = JSON.parse(event.detail)
    return parsed && typeof parsed === 'object'
      ? (parsed as Record<string, unknown>)
      : null
  } catch {
    return null
  }
}

function isStep(event: AgentEvent, title: string) {
  return (
    event.type === 'toolCall' &&
    event.id.startsWith('control-') &&
    event.title === title
  )
}

/** The conversation showing now, and the earlier ones you can resume (newest first). */
export function splitConversations(events: TimedEvent[]): {
  current: Conversation
  earlier: Conversation[]
} {
  const all = new Map<string, Conversation>()
  const open = (id: string, atMs: number): Conversation => {
    const existing = all.get(id)
    if (existing) return existing
    const fresh: Conversation = {
      id,
      events: [],
      firstPrompt: null,
      prompts: 0,
      lastAtMs: atMs,
      sessionId: null,
    }
    all.set(id, fresh)
    return fresh
  }
  let current = open('first', events[0]?.atMs ?? 0)

  for (const row of events) {
    const { event } = row
    const starting = isStep(event, NEW_SESSION_TITLE)
    const resuming = isStep(event, RESUME_TITLE)
    if (starting || resuming) {
      const detail = stepDetail(event)
      const ended = detail?.endedSession
      if (typeof ended === 'string') current.sessionId = ended
      const target = resuming ? detail?.conversation : null
      current =
        typeof target === 'string' && all.has(target)
          ? all.get(target)!
          : open(`after-${row.seq}`, row.atMs)
      continue
    }
    current.events.push(row)
    current.lastAtMs = row.atMs
    if (event.type === 'messageChunk' && event.role === 'user') {
      current.prompts += 1
      current.firstPrompt ??= event.text.trim() || null
    }
  }

  const earlier = [...all.values()]
    .filter((conversation) => conversation !== current && conversation.prompts)
    .sort((a, b) => b.lastAtMs - a.lastAtMs)
  return { current, earlier }
}

const HERO_LINES = [
  'What are we building?',
  'What should we ship today?',
  'Where do we start?',
  'What’s next?',
  'Got a bug to squash?',
  'What are we making better?',
  'What’s on your mind?',
  'Ready when you are.',
]

/** A greeting for an empty conversation; the same one for as long as it stays empty. */
export function heroLine(seed: string): string {
  let hash = 0
  for (const char of seed) hash = (hash * 31 + char.charCodeAt(0)) | 0
  return HERO_LINES[Math.abs(hash) % HERO_LINES.length]!
}
