import { commands, type CoreError } from '$lib/ipc'
import { refreshSnapshot } from '$lib/core/sync'
import { toastCoreError } from '$lib/feedback/wire-feedback'

/** Runs a git command, toasting its error, then refreshes; resolves to whether it succeeded. */
async function runGit(
  action: () => Promise<
    { status: 'ok' } | { status: 'error'; error: CoreError }
  >,
): Promise<boolean> {
  const result = await action()
  if (result.status === 'error') toastCoreError(result.error)
  await refreshSnapshot()
  return result.status === 'ok'
}

/** Merges or rebases the base branch in, as the repo is set up to. */
export function pullWorkspace(workspaceId: string) {
  return runGit(() => commands.pullWorkspace(workspaceId))
}

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

/** Adds a branch on top of the stack. A name the core refuses comes back to show by the field;
 * any other failure toasts. */
export async function addStackBranch(
  workspaceId: string,
  branch: string,
): Promise<{ added: boolean; refused: string | null }> {
  const result = await commands.addStackBranch({ workspaceId, branch })
  if (result.status === 'error' && result.error.kind === 'Invalid') {
    return { added: false, refused: result.error.message }
  }
  if (result.status === 'error') toastCoreError(result.error)
  await refreshSnapshot()
  return { added: result.status === 'ok', refused: null }
}

export async function pushStack(workspaceId: string) {
  return runGit(() => commands.pushStack(workspaceId))
}

export async function syncStack(workspaceId: string) {
  return runGit(() => commands.syncStack(workspaceId))
}
