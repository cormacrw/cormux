// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type {
  RepoGitRuntime,
  RepoRecord,
  RepoRunCommand,
} from '$lib/ipc/bindings'
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

  /** Each repo's run command as the core resolves it (its own, else its committed config's). */
  runCommandById = $state.raw<Record<string, RepoRunCommand>>({})

  /** `runCommands` replaces the known run commands; without it they stay as they are. */
  hydrate(items: RepoRecord[], runCommands?: RepoRunCommand[]) {
    const changed = repoKey(items) !== repoKey(this.items)
    this.items = reuseRows(this.items, items)
    if (runCommands) {
      this.runCommandById = Object.fromEntries(
        runCommands.map((row) => [row.repoId, row]),
      )
    }
    if (changed) void this.refreshGit()
  }

  getById(id: string) {
    return this.byId.get(id)
  }

  /** What Run starts in the repo's workspaces unless one sets its own; '' for nothing. */
  runCommand(id: string | undefined): string {
    return (id && this.runCommandById[id]?.command) || ''
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
