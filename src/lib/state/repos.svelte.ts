// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type {
  RepoCommittedSetup,
  RepoGitRuntime,
  RepoRecord,
  RepoRemovalBlock,
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

  /** The setup each repo's `.cormux/config.json` commits, which runs where Settings' is blank. */
  committedSetupById = $state.raw<Record<string, string>>({})

  private gitRead = false

  /** Why Remove would be refused now, by repo id; the core's own words. */
  removalBlockById = $state.raw<Record<string, string>>({})

  /** `snapshot` replaces the commands and removal blocks; without it they stay as they are. */
  hydrate(
    items: RepoRecord[],
    snapshot?: {
      runCommands: RepoRunCommand[]
      committedSetups: RepoCommittedSetup[]
      removalBlocks: RepoRemovalBlock[]
    },
  ) {
    this.items = reuseRows(this.items, items)
    if (snapshot) {
      this.runCommandById = Object.fromEntries(
        snapshot.runCommands.map((row) => [row.repoId, row]),
      )
      this.committedSetupById = Object.fromEntries(
        snapshot.committedSetups.map((row) => [row.repoId, row.setupCommands]),
      )
      this.removalBlockById = Object.fromEntries(
        snapshot.removalBlocks.map((row) => [row.repoId, row.reason]),
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

  /** The repo's default branch, as the core resolved it; '' for a repo that isn't registered. */
  defaultBranch(id: string): string {
    return this.byId.get(id)?.defaultBranch ?? ''
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
  repos.hydrate(snapshot.persisted.repos, {
    runCommands: snapshot.repoRunCommands,
    committedSetups: snapshot.repoCommittedSetups,
    removalBlocks: snapshot.repoRemovalBlocks,
  }),
)
onChange((change) => {
  if (change === 'gap' || change.repoGit) void repos.refreshGit()
})
