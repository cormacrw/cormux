import { describe, expect, it, vi } from 'vitest'
import type { Component } from 'svelte'
import { loadView } from './lazy-views'

describe('loadView', () => {
  it('resolves the component', async () => {
    const View = (() => {}) as unknown as Component
    const report = vi.fn()
    await expect(
      loadView(() => Promise.resolve({ default: View }), report),
    ).resolves.toBe(View)
    expect(report).not.toHaveBeenCalled()
  })

  it('reports a chunk that fails to load instead of rejecting', async () => {
    const failure = new Error('Failed to fetch dynamically imported module')
    const report = vi.fn()
    await expect(
      loadView(() => Promise.reject(failure), report),
    ).resolves.toBeUndefined()
    expect(report).toHaveBeenCalledWith(failure)
  })
})
