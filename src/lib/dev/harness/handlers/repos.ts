import { fixtureSnapshot } from '../../fixture-snapshot'
import type { HarnessRuntime } from '../runtime'
import type { HandlerSlice, Input } from '../types'

export function repoHandlers({ emitStateChanged }: HarnessRuntime) {
  const findRepo = (repoId: string) =>
    fixtureSnapshot.persisted.repos.find((row) => row.id === repoId)

  return {
    // Like the core: save the repo, then announce a state change.
    set_repo_run_command: ({ input }: Input<'set_repo_run_command'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.runCommand = input.runCommand?.trim() || null
      emitStateChanged()
      return null
    },
    set_repo_setup_commands: ({ input }: Input<'set_repo_setup_commands'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.setupCommands = input.setupCommands ?? ''
      emitStateChanged()
      return null
    },
    set_repo_single_instance: ({
      input,
    }: Input<'set_repo_single_instance'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.singleInstance = input.singleInstance ?? false
      emitStateChanged()
      return null
    },
    set_repo_default_branch: ({ input }: Input<'set_repo_default_branch'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.defaultBranch = input.defaultBranch
      return null
    },
    list_repo_skills: () => [
      { name: 'deploy', description: 'Ship it', source: 'project' },
      { name: 'review', description: 'Review the diff', source: 'user' },
    ],
  } satisfies HandlerSlice
}
