import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const HOUR_MS = 60 * 60 * 1000

// A laptop asleep: the clock moves on, but no timer fires.
function sleep(ms: number) {
  vi.setSystemTime(Date.now() + ms)
}

async function loadStore() {
  vi.resetModules()
  const { snoozes } = await import('./snooze-store.svelte')
  return snoozes
}

describe('snoozes', () => {
  let win: EventTarget
  let doc: EventTarget & { visibilityState: string }

  beforeEach(() => {
    vi.useFakeTimers()
    win = new EventTarget()
    doc = Object.assign(new EventTarget(), { visibilityState: 'visible' })
    vi.stubGlobal('window', win)
    vi.stubGlobal('document', doc)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('ends on time while the machine stays awake', async () => {
    const snoozes = await loadStore()
    snoozes.snooze('t1', Date.now() + HOUR_MS)
    vi.advanceTimersByTime(HOUR_MS - 1000)
    expect(snoozes.until).toHaveProperty('t1')
    vi.advanceTimersByTime(2000)
    expect(snoozes.until).not.toHaveProperty('t1')
  })

  it('ends as soon as the window is focused after sleeping past it', async () => {
    const snoozes = await loadStore()
    snoozes.snooze('t1', Date.now() + HOUR_MS)
    sleep(2 * HOUR_MS)
    expect(snoozes.until).toHaveProperty('t1')
    win.dispatchEvent(new Event('focus'))
    expect(snoozes.until).not.toHaveProperty('t1')
    expect(snoozes.nowMs).toBe(Date.now())
  })

  it('ends when a hidden window becomes visible again', async () => {
    const snoozes = await loadStore()
    snoozes.snooze('t1', Date.now() + HOUR_MS)
    sleep(2 * HOUR_MS)
    doc.dispatchEvent(new Event('visibilitychange'))
    expect(snoozes.until).not.toHaveProperty('t1')
  })

  it('catches up within a minute even without a focus change', async () => {
    const snoozes = await loadStore()
    snoozes.snooze('t1', Date.now() + HOUR_MS)
    sleep(2 * HOUR_MS)
    vi.advanceTimersByTime(61 * 1000)
    expect(snoozes.until).not.toHaveProperty('t1')
  })

  it('keeps a snooze that has not run out', async () => {
    const snoozes = await loadStore()
    snoozes.snooze('t1', Date.now() + 3 * HOUR_MS)
    sleep(HOUR_MS)
    win.dispatchEvent(new Event('focus'))
    expect(snoozes.until).toHaveProperty('t1')
  })
})
