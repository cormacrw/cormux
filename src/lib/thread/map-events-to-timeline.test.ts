import { describe, expect, it, beforeEach } from 'vitest'
import {
  mapEventsToTimeline,
  resetTimelineIdCounter,
  shouldShowLiveRow,
} from './map-events-to-timeline'

describe('mapEventsToTimeline', () => {
  beforeEach(() => {
    resetTimelineIdCounter()
  })

  it('coalesces user and agent message chunks', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: { type: 'messageChunk', role: 'user', text: 'Hello ' },
        },
        {
          seq: 2,
          event: { type: 'messageChunk', role: 'user', text: 'world' },
        },
        {
          seq: 3,
          event: { type: 'messageChunk', role: 'thought', text: 'Thinking…' },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
    })
    expect(items).toHaveLength(2)
    expect(items[0]).toMatchObject({ kind: 'user', text: 'Hello world' })
    expect(items[1]).toMatchObject({ kind: 'thought', text: 'Thinking…' })
  })

  it('groups consecutive tool calls into one run', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 't1',
            title: 'Read files',
            name: null,
            kind: 'read',
            status: 'completed',
            locations: ['src/a.ts'],
            detail: null,
          },
        },
        {
          seq: 2,
          event: {
            type: 'toolCall',
            id: 't2',
            title: 'Grep',
            name: null,
            kind: 'search',
            status: 'completed',
            locations: [],
            detail: 'pattern',
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
    })
    expect(items).toHaveLength(1)
    const run = items[0]
    expect(run?.kind).toBe('toolRun')
    if (run?.kind === 'toolRun') {
      expect(run.steps).toHaveLength(2)
    }
  })

  it('maps edit tool calls to edit steps', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'e1',
            title: 'Write',
            name: null,
            kind: 'edit',
            status: 'completed',
            locations: ['src/lib/auth.ts'],
            detail: null,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
    })
    const run = items[0]
    expect(run?.kind).toBe('toolRun')
    if (run?.kind === 'toolRun') {
      expect(run.steps[0]).toMatchObject({
        kind: 'edit',
        path: 'src/lib/auth.ts',
      })
    }
  })

  it('maps permission rows using stored approval payload', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'permission',
            id: 'perm-1',
            tool_call_id: null,
            title: 'Use Bash',
            tool_name: 'Bash',
            kind: 'execute',
            detail: 'pnpm test',
            auto_approved: false,
          },
        },
      ],
      approvals: [
        {
          id: 'perm-1',
          threadId: 't1',
          status: 'pending',
          tool: 'execute',
          payload: JSON.stringify({
            title: 'Run command',
            what: 'pnpm test',
            why: 'Bash',
            okLabel: 'Approve',
            noLabel: 'Deny',
          }),
        },
      ],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
    })
    expect(items[0]).toMatchObject({
      kind: 'approval',
      title: 'Run command',
      okLabel: 'Approve',
    })
  })

  it('inserts items before live row', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: { type: 'messageChunk', role: 'user', text: 'Go' },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: true,
      liveToolTitle: null,
    })
    expect(items.at(-1)?.kind).toBe('live')
    expect(items[0]?.kind).toBe('user')
  })
})

describe('shouldShowLiveRow', () => {
  it('shows live while running or provisioning', () => {
    expect(shouldShowLiveRow('running', false)).toBe(true)
    expect(shouldShowLiveRow('provisioning', false)).toBe(true)
    expect(shouldShowLiveRow('idle', false)).toBe(false)
  })
})
