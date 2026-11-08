import { describe, expect, it } from 'vitest'
import {
  placeKey,
  placeOf,
  startIndex,
  stepIndex,
  switcherPlaces,
  visit,
  type Place,
} from './places'

const ws = (id: string): Place => ({ kind: 'workspace', id })

describe('visit', () => {
  it('moves the place to the front without duplicating it', () => {
    let history: Place[] = []
    history = visit(history, ws('a'))
    history = visit(history, { kind: 'newThread' })
    history = visit(history, ws('b'))
    history = visit(history, ws('a'))
    expect(history.map(placeKey)).toEqual([
      'workspace:a',
      'workspace:b',
      'newThread',
    ])
  })
})

describe('switcherPlaces', () => {
  it('lists recent places first, then everything else', () => {
    const history = [ws('b'), { kind: 'newThread' } as Place, ws('a')]
    expect(
      switcherPlaces(history, {
        workspaceIds: ['a', 'b', 'c'],
        sprintAvailable: true,
      }).map(placeKey),
    ).toEqual([
      'workspace:b',
      'newThread',
      'workspace:a',
      'workspace:c',
      'sprint',
    ])
  })

  it('leaves out deleted workspaces and a Sprint page that isn’t set up', () => {
    const history = [ws('gone'), { kind: 'sprint' } as Place, ws('a')]
    expect(
      switcherPlaces(history, {
        workspaceIds: ['a'],
        sprintAvailable: false,
      }).map(placeKey),
    ).toEqual(['workspace:a', 'newThread'])
  })
})

describe('selection', () => {
  const list = [ws('b'), { kind: 'newThread' } as Place, ws('a')]

  it('starts on the previous place when you’re on the newest', () => {
    expect(startIndex(list, ws('b'))).toBe(1)
    // From a page the switcher doesn't list (Settings), the newest place comes first.
    expect(startIndex(list, null)).toBe(0)
    expect(startIndex([ws('b')], ws('b'))).toBe(0)
  })

  it('wraps around both ends', () => {
    expect(stepIndex(2, 3, 1)).toBe(0)
    expect(stepIndex(0, 3, -1)).toBe(2)
  })
})

describe('placeOf', () => {
  it('knows workspaces and the main pages, not Settings, scratches or repos', () => {
    expect(
      placeOf({ view: 'workspace', workspaceId: 'ws-1', threadId: null }),
    ).toEqual(ws('ws-1'))
    expect(placeOf({ view: 'sprint' })).toEqual({ kind: 'sprint' })
    expect(placeOf({ view: 'newThread' })).toEqual({ kind: 'newThread' })
    expect(placeOf({ view: 'settings' })).toBeNull()
    expect(placeOf({ view: 'scratch', scratchId: 's-1' })).toBeNull()
    expect(placeOf({ view: 'repo', repoId: 'r-1' })).toBeNull()
  })
})
