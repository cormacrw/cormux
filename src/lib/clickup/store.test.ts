import { describe, expect, it, vi } from 'vitest'
import type { ClickupBoard } from '$lib/ipc/bindings'

const clickupBoard = vi.fn()

vi.mock('$lib/ipc', () => ({ commands: { clickupBoard } }))
vi.mock('$lib/core/sync', () => ({ onSnapshot: () => () => {} }))

const { ClickupStore } = await import('./store.svelte')

const board = (name: string) =>
  ({ statuses: [], tasks: [], name }) as unknown as ClickupBoard

describe('ClickupStore.hydrate', () => {
  it('starts the board over for a new folder from the snapshot alone', async () => {
    clickupBoard.mockResolvedValueOnce({ status: 'ok', data: board('one') })
    const store = new ClickupStore()
    store.hydrate(true, 'folder-1')
    await vi.waitFor(() => expect(store.board).not.toBeNull())

    clickupBoard.mockResolvedValueOnce({ status: 'ok', data: board('two') })
    store.selectedTaskId = 'task'
    store.hydrate(true, 'folder-2')

    expect(store.selectedTaskId).toBeNull()
    await vi.waitFor(() => expect(store.board).toEqual(board('two')))
    expect(clickupBoard).toHaveBeenCalledTimes(2)
  })
})
