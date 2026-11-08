import { describe, expect, it, vi } from 'vitest'
import type { Settings, Snapshot } from '$lib/ipc/bindings'
import { defaultSettings } from './defaults'

const core = vi.hoisted(() => ({
  settings: null as Settings | null,
  version: 1,
  fetches: [] as (() => Promise<Snapshot>)[],
}))

vi.mock(import('$lib/ipc/bindings'), async (importOriginal) => ({
  ...(await importOriginal()),
  commands: {} as never,
  events: {} as never,
}))
vi.mock('$lib/ipc', () => ({
  commands: {
    updateSettings: async (patch: Partial<Settings>) => {
      core.settings = { ...core.settings!, ...patch }
      core.version += 1
      return { status: 'ok', data: core.settings }
    },
  },
}))
vi.mock(import('$lib/core/sync'), async (importOriginal) => {
  const { createSync } = await importOriginal()
  return { ...createSync(() => core.fetches.shift()!()), createSync }
})

const { settings } = await import('./store.svelte')
const { refreshSnapshot } = await import('$lib/core/sync')

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => (resolve = done))
  return { promise, resolve }
}

const snapshotNow = () =>
  ({ version: core.version, settings: core.settings }) as Snapshot

describe('settings.update', () => {
  it('is not rolled back by a snapshot fetched before the save', async () => {
    core.settings = { ...defaultSettings(), clickupFolderId: '' }
    const stale = deferred<Snapshot>()
    const staleSnapshot = snapshotNow()
    core.fetches.push(
      () => stale.promise,
      async () => snapshotNow(),
    )

    const inFlight = refreshSnapshot()
    const saved = settings.update({ clickupFolderId: 'sprints' })
    await Promise.resolve()
    stale.resolve(staleSnapshot)

    expect(await saved).toBeNull()
    await inFlight
    expect(settings.values.clickupFolderId).toBe('sprints')
  })
})
