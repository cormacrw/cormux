import { coreErrorText } from '$lib/feedback/core-error'
import { showToast } from '$lib/feedback/show-toast'
import { commands, fetchSnapshot } from '$lib/ipc'
import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
import { bangPromptWithExtra } from '$lib/settings/bangs'
import { hydrateFromSnapshot } from '$lib/state'
import { app } from '$lib/state/app.svelte'
import { repos } from '$lib/state/repos.svelte'
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

/**
 * Starts a scratch from a bang in the default repo without leaving the current view.
 * `extra` is what was typed after `!name` in the palette, added to the prompt.
 */
export async function runBang(bangId: string, extra = '') {
  const bang = settings.bangs.find((row) => row.id === bangId)
  if (!bang) return
  const title = `!${bang.name}`
  const result = await commands.createScratch({
    title,
    repoId: resolveDefaultRepoId(repos.items, settings.defaultRepo),
    prompt: bangPromptWithExtra(bang.prompt, extra),
  })
  if (result.status === 'error') {
    showToast({
      tone: 'bad',
      parts: [
        {
          type: 'text',
          value: coreErrorText(result.error, 'Could not start the scratch'),
        },
      ],
    })
    return
  }
  hydrateFromSnapshot(await fetchSnapshot())
  showToast({
    tone: 'ok',
    parts: [{ type: 'text', value: `Started ${title}` }],
    scratchId: result.data.scratchId,
  })
}
