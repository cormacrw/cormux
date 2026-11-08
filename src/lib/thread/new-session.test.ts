import { describe, expect, it } from 'vitest'
import type { TranscriptEvent } from '$lib/ipc/bindings'
import { hasSessionToClear } from './new-session'

const marker: TranscriptEvent = {
  type: 'step',
  id: 'step-1',
  title: 'Started a new session',
  detail: null,
  step: { kind: 'sessionStarted', ended_session: null },
}
const message: TranscriptEvent = {
  type: 'messageChunk',
  role: 'user',
  text: 'hi',
}

describe('new session', () => {
  it('has nothing to clear on an empty thread or right after a new session', () => {
    expect(hasSessionToClear([])).toBe(false)
    expect(hasSessionToClear([{ event: message }, { event: marker }])).toBe(
      false,
    )
    expect(hasSessionToClear([{ event: marker }, { event: message }])).toBe(
      true,
    )
  })
})
