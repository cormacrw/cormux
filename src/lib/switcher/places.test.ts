import { describe, expect, it } from 'vitest'
import {
  placeKey,
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
    history = visit(history, { kind: 'week' })
    history = visit(history, ws('b'))
    history = visit(history, ws('a'))
    expect(history.map(placeKey)).toEqual([
      'workspace:a',
      'workspace:b',
      'week',
    ])
  })
})

describe('switcherPlaces', () => {
  it('lists recent places first, then everything else', () => {
    const history = [ws('b'), { kind: 'week' } as Place, ws('a')]
    expect(
      switcherPlaces(history, {
        workspaceIds: ['a', 'b', 'c'],
        sprintAvailable: true,
      }).map(placeKey),
    ).toEqual([
      'workspace:b',
      'week',
      'workspace:a',
      'workspace:c',
      'homebase',
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
    ).toEqual(['workspace:a', 'homebase', 'week'])
  })
})

describe('selection', () => {
  const list = [ws('b'), { kind: 'week' } as Place, ws('a')]

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
