import type { RepoRecord } from '$lib/ipc/bindings'

export class ReposStore {
  items = $state<RepoRecord[]>([])

  hydrate(items: RepoRecord[]) {
    this.items = items
  }

  getById(id: string) {
    return this.items.find((repo) => repo.id === id)
  }
}

export const repos = new ReposStore()
