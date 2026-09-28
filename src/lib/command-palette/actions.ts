import { app } from '$lib/state/app.svelte'
import { settings } from '$lib/state/settings.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'

/** Hooks for COR-15 / COR-17 — palette closes before these run. */
export function runWorkspaceApp(
  workspaceId: string,
  action: 'run' | 'restart' | 'stop' | 'clear',
) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-app', {
      detail: { workspaceId, action },
    }),
  )
}

export function openRepoRunCommand(repoId: string) {
  settings.focusSection = 'repos'
  settings.expandRepoId = repoId
  settings.focusRepoRunCommand = repoId
  app.openSettings()
}

export function pullWorkspace(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-pull', { detail: { workspaceId } }),
  )
}

export function rebaseWorkspace(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-rebase', { detail: { workspaceId } }),
  )
}

export function pushWorkspace(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-push', { detail: { workspaceId } }),
  )
}

export function abortWorkspaceGit(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-git-abort', { detail: { workspaceId } }),
  )
}

export function requestNewThread(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:new-thread', { detail: { workspaceId } }),
  )
}

export function openSettingsSection(section: string) {
  settings.focusSection = section
  app.openSettings()
}

export function openWorkspaceFindings(workspaceId: string) {
  app.openWorkspace(workspaceId)
  workspaceUi.findingsFocusPending = true
  workspaceUi.openTab('findings')
}

export function openWorkspaceThread(workspaceId: string, threadId: string) {
  app.openWorkspace(workspaceId, threadId)
  workspaceUi.openTab('thread')
}

export function toggleReduceMotion() {
  settings.toggleReduceMotion()
}
