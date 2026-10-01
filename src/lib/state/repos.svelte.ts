import type { RepoGitRuntime, RepoRecord } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'

export class ReposStore {
  items = $state<RepoRecord[]>([])
  /** Default branch vs `origin`, by repo id; missing until the first read. */
  gitById = $state<Record<string, RepoGitRuntime>>({})

  hydrate(items: RepoRecord[]) {
    const changed = repoKey(items) !== repoKey(this.items)
    this.items = items
    if (changed) void this.refreshGit()
  }

  getById(id: string) {
    return this.items.find((repo) => repo.id === id)
  }

  /** Re-reads the counts; call after a fetch or pull moves `origin` or the branch. */
  async refreshGit() {
    const result = await commands.getRepoGit()
    if (result.status !== 'ok') return
    this.gitById = Object.fromEntries(
      result.data.map((row) => [row.repoId, row]),
    )
  }
}

const repoKey = (items: RepoRecord[]) =>
  items.map((repo) => `${repo.id}:${repo.defaultBranch ?? ''}`).join('\n')

export const repos = new ReposStore()
