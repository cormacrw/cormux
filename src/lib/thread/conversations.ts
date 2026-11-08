import type { TranscriptEvent } from '$lib/ipc/bindings'
import type { TimedEvent } from './timed-events'

/** The conversation showing now: the one the newest event is in (the core says which). */
export type CurrentConversation = { id: string; events: TimedEvent[] }

/** A session step: it begins a conversation, so it isn't shown in one. */
function isSessionStep(event: TranscriptEvent) {
  return (
    event.type === 'step' &&
    (event.step.kind === 'sessionStarted' ||
      event.step.kind === 'sessionResumed')
  )
}

export function currentConversation(events: TimedEvent[]): CurrentConversation {
  const id = events.at(-1)?.conversation ?? 'first'
  return {
    id,
    events: events.filter(
      (row) => row.conversation === id && !isSessionStep(row.event),
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
