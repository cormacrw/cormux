import { coreErrorText } from '$lib/feedback/core-error'
import { showToast } from '$lib/feedback/show-toast'
import { commands, fetchSnapshot } from '$lib/ipc'
import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
import { macroPromptWithExtra } from '$lib/settings/scratch-macros'
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

/**
 * Starts a scratch from a macro in the default repo without leaving the current view.
 * `extra` is what was typed after the macro chip in the palette.
 */
export async function runScratchMacro(macroId: string, extra = '') {
  const macro = settings.scratchMacros.find((row) => row.id === macroId)
  if (!macro) return
  const title = macro.name.trim()
  const result = await commands.createScratch({
    title,
    repoId: resolveDefaultRepoId(repos.items, settings.defaultRepo),
    prompt: macroPromptWithExtra(macro.prompt, extra),
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
