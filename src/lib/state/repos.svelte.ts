// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { RepoGitRuntime, RepoRecord } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import { reuseRows } from './reuse-rows'

export class ReposStore {
  // Raw and reused: nearly every view reads the repo list, so an unchanged one must stay the same object.
  items = $state.raw<RepoRecord[]>([])
  /** Default branch vs `origin`, by repo id; missing until the first read. */
  gitById = $state.raw<Record<string, RepoGitRuntime>>({})

  private readonly byId = $derived(
    new Map(this.items.map((repo) => [repo.id, repo])),
  )

  hydrate(items: RepoRecord[]) {
    const changed = repoKey(items) !== repoKey(this.items)
    this.items = reuseRows(this.items, items)
    if (changed) void this.refreshGit()
  }

  getById(id: string) {
    return this.byId.get(id)
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
