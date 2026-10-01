import { commands, fetchSnapshot, type CoreError } from '$lib/ipc'
import { hydrateFromSnapshot } from '$lib/state'
import { coreErrorText } from '$lib/feedback/core-error'
import { toastCoreError } from '$lib/feedback/wire-feedback'

/** Runs a git action, toasting its error; resolves to whether it succeeded. */
async function runGit(
  action: () => Promise<{ status: string; error?: CoreError }>,
): Promise<boolean> {
  const result = await action()
  if (result.status === 'ok') {
    hydrateFromSnapshot(await fetchSnapshot())
    return true
  }
  if (result.error) {
    toastCoreError(coreErrorText(result.error))
    hydrateFromSnapshot(await fetchSnapshot())
  }
  return false
}

export function bindGitWorkspaceControls() {
  const onPull = (event: Event) => {
    const detail = (event as CustomEvent).detail as { workspaceId: string }
    void runGit(() => commands.pullWorkspace(detail.workspaceId))
  }

  const onAbort = (event: Event) => {
    const detail = (event as CustomEvent).detail as { workspaceId: string }
    void runGit(() => commands.abortWorkspaceGit(detail.workspaceId))
  }

  window.addEventListener('cormux:workspace-pull', onPull)
  window.addEventListener('cormux:workspace-git-abort', onAbort)

  return () => {
    window.removeEventListener('cormux:workspace-pull', onPull)
    window.removeEventListener('cormux:workspace-git-abort', onAbort)
  }
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
