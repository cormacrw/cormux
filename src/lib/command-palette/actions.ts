import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
import { bangPromptWithExtra } from '$lib/settings/bangs'
import { startReviewWorkspace } from '$lib/review/start-review'
import { app } from '$lib/state/app.svelte'
import type { PullRequest } from '$lib/state/prs.svelte'
import { repos } from '$lib/state/repos.svelte'
import { settings } from '$lib/state/settings.svelte'
import { workspaceRecords } from '$lib/state/workspace-records.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { workspaces } from '$lib/state/workspaces.svelte'
import { startThread } from '$lib/workspace/start-thread'

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

export function abortWorkspaceGit(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-git-abort', { detail: { workspaceId } }),
  )
}

/** One review workspace per PR: reopens it rather than starting a duplicate. */
export function reviewPullRequest(pr: PullRequest) {
  const existing = workspaces.items.find(
    (row) =>
      row.kind === 'review' &&
      row.prNumber === pr.num &&
      (!pr.repoId || workspaceRecords.getRecord(row.id)?.repoId === pr.repoId),
  )
  if (existing) app.openWorkspace(existing.id)
  else void startReviewWorkspace(pr)
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

/**
 * Starts a thread from a bang in the default repo without leaving the current view.
 * `extra` is what was typed after `!name` in the palette, added to the prompt.
 */
export async function runBang(bangId: string, extra = '') {
  const bang = settings.bangs.find((row) => row.id === bangId)
  if (!bang) return
  await startThread({
    repoId: resolveDefaultRepoId(repos.items, settings.defaultRepo),
    base: null,
    prompt: bangPromptWithExtra(bang.prompt, extra),
  })
}
