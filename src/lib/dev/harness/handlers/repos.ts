import { CHANGE, type HarnessRuntime } from '../runtime'
import type { HandlerSlice, Input, Unsupported } from '../types'

export const repoUnsupported = [
  'add_repo',
  'remove_repo',
  'test_repo_setup',
] as const satisfies Unsupported

export function repoHandlers({ db, emitStateChanged }: HarnessRuntime) {
  const findRepo = (repoId: string) =>
    db.snapshot.persisted.repos.find((row) => row.id === repoId)

  return {
    set_repo_run_command: ({ input }: Input<'set_repo_run_command'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.runCommand = input.runCommand?.trim() || null
      emitStateChanged(CHANGE.state)
      return null
    },
    set_repo_setup_commands: ({ input }: Input<'set_repo_setup_commands'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.setupCommands = input.setupCommands ?? ''
      emitStateChanged(CHANGE.state)
      return null
    },
    set_repo_single_instance: ({
      input,
    }: Input<'set_repo_single_instance'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.singleInstance = input.singleInstance ?? false
      emitStateChanged(CHANGE.state)
      return null
    },
    set_repo_default_branch: ({ input }: Input<'set_repo_default_branch'>) => {
      const repo = findRepo(input.repoId)
      if (repo) repo.defaultBranch = input.defaultBranch
      emitStateChanged(CHANGE.repoGit)
      return null
    },
    list_repo_skills: () => [
      { name: 'deploy', description: 'Ship it', source: 'project' },
      { name: 'review', description: 'Review the diff', source: 'user' },
    ],
  } satisfies HandlerSlice
}
