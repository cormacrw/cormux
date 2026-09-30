import { describe, expect, it } from 'vitest'
import type { AgentEvent } from '$lib/ipc/bindings'
import { mapEventsToTimeline } from './map-events-to-timeline'
import { hasSessionToClear, NEW_SESSION_TITLE } from './new-session'

const marker: AgentEvent = {
  type: 'toolCall',
  id: 'control-1',
  title: NEW_SESSION_TITLE,
  name: null,
  kind: 'other',
  status: 'completed',
  locations: [],
  detail: null,
}
const message: AgentEvent = { type: 'messageChunk', role: 'user', text: 'hi' }

describe('new session', () => {
  it('renders the marker as an app event divider', () => {
    const items = mapEventsToTimeline({
      events: [{ seq: 1, atMs: 0, event: marker }],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
    })
    expect(items).toMatchObject([
      { kind: 'event', icon: 'session', title: NEW_SESSION_TITLE },
    ])
  })

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
