import { commands, fetchSnapshot, type CoreError } from '$lib/ipc'
import { hydrateFromSnapshot } from '$lib/state'
import { coreErrorText } from '$lib/feedback/core-error'
import { toastCoreError } from '$lib/feedback/wire-feedback'

async function runGit(action: () => Promise<{ status: string; error?: CoreError }>) {
  const result = await action()
  if (result.status === 'ok') {
    hydrateFromSnapshot(await fetchSnapshot())
    return
  }
  if (result.error) {
    toastCoreError(coreErrorText(result.error))
    hydrateFromSnapshot(await fetchSnapshot())
  }
}

export function bindGitWorkspaceControls() {
  const onPull = (event: Event) => {
    const detail = (event as CustomEvent).detail as { workspaceId: string }
    void runGit(() => commands.pullWorkspace(detail.workspaceId))
  }

  const onRebase = (event: Event) => {
    const detail = (event as CustomEvent).detail as { workspaceId: string }
    void runGit(() => commands.rebaseWorkspace(detail.workspaceId))
  }

  const onPush = (event: Event) => {
    const detail = (event as CustomEvent).detail as { workspaceId: string }
    void runGit(() => commands.pushWorkspaceBranch(detail.workspaceId))
  }

  const onAbort = (event: Event) => {
    const detail = (event as CustomEvent).detail as { workspaceId: string }
    void runGit(() => commands.abortWorkspaceGit(detail.workspaceId))
  }

  window.addEventListener('cormux:workspace-pull', onPull)
  window.addEventListener('cormux:workspace-rebase', onRebase)
  window.addEventListener('cormux:workspace-push', onPush)
  window.addEventListener('cormux:workspace-git-abort', onAbort)

  return () => {
    window.removeEventListener('cormux:workspace-pull', onPull)
    window.removeEventListener('cormux:workspace-rebase', onRebase)
    window.removeEventListener('cormux:workspace-push', onPush)
    window.removeEventListener('cormux:workspace-git-abort', onAbort)
  }
}

export async function switchWorkspaceBranch(workspaceId: string, branch: string) {
  await runGit(() =>
    commands.switchWorkspaceBranch({ workspaceId, branch }),
  )
}

export async function createWorkspaceBranch(workspaceId: string, branch: string) {
  await runGit(() =>
    commands.createWorkspaceBranch({ workspaceId, branch }),
  )
}
