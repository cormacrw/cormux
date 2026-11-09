import {
  CONVERSATION_STEP_KINDS,
  FIRST_CONVERSATION,
  TURN_ENDING_EVENTS,
  type TimelineStep,
  type TranscriptEvent,
} from '$lib/ipc/bindings'
import type { TimedEvent } from './timed-events'

/** The conversation showing now: the one the newest event is in (the core says which). */
export type CurrentConversation = { id: string; events: TimedEvent[] }

type SessionStep = Extract<
  TimelineStep,
  { kind: (typeof CONVERSATION_STEP_KINDS)[number] }
>

/** A step that begins a conversation, by the core's list. */
export function isSessionStep(step: TimelineStep): step is SessionStep {
  return (CONVERSATION_STEP_KINDS as readonly string[]).includes(step.kind)
}

/** An event after which the thread's turn is over, by the core's list. */
export function endsTurn(event: TranscriptEvent): boolean {
  return (TURN_ENDING_EVENTS as readonly string[]).includes(event.type)
}

export function currentConversation(events: TimedEvent[]): CurrentConversation {
  const id = events.at(-1)?.conversation ?? FIRST_CONVERSATION
  return {
    id,
    events: events.filter(
      // A session step begins a conversation, so it isn't shown in one.
      (row) =>
        row.conversation === id &&
        !(row.event.type === 'step' && isSessionStep(row.event.step)),
    ),
  }
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
