import { describe, expect, it, vi } from 'vitest'
import type { Snapshot } from '$lib/ipc/bindings'

vi.mock('$lib/ipc/bindings', () => ({ commands: {}, events: {} }))

const { createSync } = await import('./sync')

const snapshot = (version: number) => ({ version }) as Snapshot

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => (resolve = done))
  return { promise, resolve }
}

const NO_CHANGE = { repoGit: false }

describe('sync', () => {
  it('drops a snapshot older than the one already applied', async () => {
    const fetches = [snapshot(5), snapshot(3)]
    const sync = createSync(async () => fetches.shift()!)
    const seen: number[] = []
    sync.onSnapshot((s) => seen.push(s.version))

    await sync.refreshSnapshot()
    await sync.refreshSnapshot()

    expect(seen).toEqual([5])
  })

  it('applies a snapshot at the same version, since not every change bumps it', async () => {
    const sync = createSync(async () => snapshot(4))
    const seen: number[] = []
    sync.onSnapshot((s) => seen.push(s.version))

    await sync.refreshSnapshot()
    await sync.refreshSnapshot()

    expect(seen).toEqual([4, 4])
  })

  it('gives a store that registers late the last snapshot at once', async () => {
    const sync = createSync(async () => snapshot(2))
    await sync.refreshSnapshot()

    const seen: number[] = []
    sync.onSnapshot((s) => seen.push(s.version))

    expect(seen).toEqual([2])
  })

  it('re-reads after a caller asks during a fetch, so the caller sees what it did', async () => {
    const first = deferred<Snapshot>()
    const fetch = vi
      .fn<() => Promise<Snapshot>>()
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce(snapshot(2))
    const sync = createSync(fetch)
    const seen: number[] = []
    sync.onSnapshot((s) => seen.push(s.version))

    const running = sync.refreshSnapshot()
    const asked = sync.refreshSnapshot()
    first.resolve(snapshot(1))
    await asked
    await running

    expect(fetch).toHaveBeenCalledTimes(2)
    expect(seen).toEqual([1, 2])
  })

  it('reports a gap when an event skips a version, and the change otherwise', async () => {
    const sync = createSync(async () => snapshot(3))
    await sync.refreshSnapshot()
    const changes: unknown[] = []
    sync.onChange((change) => changes.push(change))

    sync.receive({ version: 4, change: { repoGit: true } })
    sync.receive({ version: 6, change: NO_CHANGE })

    expect(changes).toEqual([{ repoGit: true }, 'gap'])
  })
})
