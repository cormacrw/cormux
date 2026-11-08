// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type {
  RepoGitRuntime,
  RepoRecord,
  RepoRunCommand,
} from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import { onChange, onSnapshot } from '$lib/core/sync'
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

  private gitRead = false

  /** `runCommands` replaces the known run commands; without it they stay as they are. */
  hydrate(items: RepoRecord[], runCommands?: RepoRunCommand[]) {
    this.items = reuseRows(this.items, items)
    if (runCommands) {
      this.runCommandById = Object.fromEntries(
        runCommands.map((row) => [row.repoId, row]),
      )
    }
    // After the first, the core says when the counts move (`repoGit`).
    if (!this.gitRead) {
      this.gitRead = true
      void this.refreshGit()
    }
  }

  getById(id: string) {
    return this.byId.get(id)
  }

  /** The repo's default branch, by the core's rule: the one set, else `main`. */
  defaultBranch(id: string): string {
    return this.byId.get(id)?.defaultBranch?.trim() || 'main'
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

export const repos = new ReposStore()

onSnapshot((snapshot) =>
  repos.hydrate(snapshot.persisted.repos, snapshot.repoRunCommands),
)
onChange((change) => {
  if (change === 'gap' || change.repoGit) void repos.refreshGit()
})
