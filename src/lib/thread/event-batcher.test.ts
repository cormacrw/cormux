import { describe, expect, it, vi } from 'vitest'
import { createEventBatcher, type BatchScheduler } from './event-batcher'

function manualScheduler() {
  const frames = new Map<number, () => void>()
  const timers = new Map<number, () => void>()
  let id = 0
  const scheduler = {
    frame: (run: () => void) => (frames.set(++id, run), id),
    cancelFrame: (key: number) => frames.delete(key),
    timer: (run: () => void) => (timers.set(++id, run), id),
    cancelTimer: (key: number) => timers.delete(key),
  } as unknown as BatchScheduler
  const fire = (from: Map<number, () => void>) =>
    [...from.values()].forEach((run) => run())
  return {
    scheduler,
    frame: () => fire(frames),
    timeout: () => fire(timers),
    pending: () => frames.size + timers.size,
  }
}

describe('createEventBatcher', () => {
  it('delivers everything from a frame at once, in order', () => {
    const clock = manualScheduler()
    const flushed = vi.fn()
    const batcher = createEventBatcher<number>(flushed, clock.scheduler)
    batcher.push(1)
    batcher.push(2)
    expect(flushed).not.toHaveBeenCalled()
    clock.frame()
    expect(flushed.mock.calls).toEqual([[[1, 2]]])
    expect(clock.pending()).toBe(0)
  })

  it('still delivers when frames have stopped', () => {
    const clock = manualScheduler()
    const flushed = vi.fn()
    const batcher = createEventBatcher<number>(flushed, clock.scheduler)
    batcher.push(1)
    clock.timeout()
    expect(flushed.mock.calls).toEqual([[[1]]])
    expect(clock.pending()).toBe(0)
  })

  it('flushes on demand and stays quiet when empty', () => {
    const clock = manualScheduler()
    const flushed = vi.fn()
    const batcher = createEventBatcher<number>(flushed, clock.scheduler)
    batcher.flush()
    expect(flushed).not.toHaveBeenCalled()
    batcher.push(1)
    batcher.flush()
    expect(flushed).toHaveBeenCalledOnce()
  })
})
