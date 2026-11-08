import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { debouncedSaves } from './debounced-saves'

describe('debouncedSaves', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('saves once typing pauses, with the latest value', () => {
    const saves = debouncedSaves(100)
    const saved: string[] = []
    saves.schedule('a', () => saved.push('first'))
    vi.advanceTimersByTime(50)
    saves.schedule('a', () => saved.push('second'))
    vi.advanceTimersByTime(99)
    expect(saved).toEqual([])
    vi.advanceTimersByTime(1)
    expect(saved).toEqual(['second'])
  })

  it('keeps keys apart, and flushes or cancels one', () => {
    const saves = debouncedSaves(100)
    const saved: string[] = []
    saves.schedule('a', () => saved.push('a'))
    saves.schedule('b', () => saved.push('b'))
    saves.flush('a')
    expect(saved).toEqual(['a'])
    expect(saves.cancel('b')).toBe(true)
    expect(saves.cancel('b')).toBe(false)
    vi.advanceTimersByTime(200)
    expect(saved).toEqual(['a'])
  })

  it('saves everything still waiting on flushAll', () => {
    const saves = debouncedSaves(100)
    const saved: string[] = []
    saves.schedule('a', () => saved.push('a'))
    saves.schedule('b', () => saved.push('b'))
    saves.flushAll()
    expect(saved).toEqual(['a', 'b'])
  })
})
