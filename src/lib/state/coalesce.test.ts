import { describe, expect, it } from 'vitest'
import { coalesce } from './coalesce'

describe('coalesce', () => {
  it('shares a run, then reruns once for everything that asked during it', async () => {
    let runs = 0
    let release: () => void = () => {}
    const refresh = coalesce(async () => {
      runs += 1
      await new Promise<void>((done) => (release = done))
    })
    const first = refresh()
    void refresh()
    void refresh()
    await Promise.resolve()
    expect(runs).toBe(1)
    release()
    await new Promise((done) => setTimeout(done))
    expect(runs).toBe(2)
    release()
    await first
    expect(runs).toBe(2)
  })
})
