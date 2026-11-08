import { commands, fetchSnapshot, type CoreError } from '$lib/ipc'
import { hydrateFromSnapshot } from '$lib/state'
import { toastCoreError } from '$lib/feedback/wire-feedback'

/** Runs a git command, toasting its error, then refreshes; resolves to whether it succeeded. */
async function runGit(
  action: () => Promise<
    { status: 'ok' } | { status: 'error'; error: CoreError }
  >,
): Promise<boolean> {
  const result = await action()
  if (result.status === 'error') toastCoreError(result.error)
  hydrateFromSnapshot(await fetchSnapshot())
  return result.status === 'ok'
}

/** Merges or rebases the base branch in, as the repo is set up to. */
export function pullWorkspace(workspaceId: string) {
  return runGit(() => commands.pullWorkspace(workspaceId))
}

/** Backs out of a conflicted merge or rebase. */
export function abortWorkspaceGit(workspaceId: string) {
  return runGit(() => commands.abortWorkspaceGit(workspaceId))
}

export async function switchWorkspaceBranch(
  workspaceId: string,
  branch: string,
) {
  return runGit(() => commands.switchWorkspaceBranch({ workspaceId, branch }))
}

export async function createWorkspaceBranch(
  workspaceId: string,
  branch: string,
) {
  await runGit(() => commands.createWorkspaceBranch({ workspaceId, branch }))
}

export async function addStackBranch(workspaceId: string, branch: string) {
  return runGit(() => commands.addStackBranch({ workspaceId, branch }))
}

export async function pushStack(workspaceId: string) {
  return runGit(() => commands.pushStack(workspaceId))
}

export async function syncStack(workspaceId: string) {
  return runGit(() => commands.syncStack(workspaceId))
}
