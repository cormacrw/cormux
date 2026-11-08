import { reviewPullRequest } from '$lib/review/start-review'
import { appControls, controlApp } from '$lib/run-app/actions'
import { openSettingsSection } from '$lib/settings/open'
import { app } from '$lib/state/app.svelte'
import { clickup } from '$lib/state/clickup.svelte'
import { repos } from '$lib/state/repos.svelte'
import { scratches } from '$lib/state/scratches.svelte'
import { settings } from '$lib/state/settings.svelte'
import { prs } from '$lib/state/prs.svelte'
import { threads } from '$lib/state/threads.svelte'
import { workspaceRecords } from '$lib/state/workspace-records.svelte'
import { workspaces } from '$lib/state/workspaces.svelte'
import { pullWorkspace } from '$lib/workspace/git-actions'
import { openWorkspaceFindings, openWorkspaceThread } from '$lib/workspace/open'
import type { PaletteCommand } from './types'
import { buildPaletteCommandsFromState } from './build-registry'
import { runBang } from './actions'

export function buildPaletteCommands(): PaletteCommand[] {
  return buildPaletteCommandsFromState(
    {
      reduceMotion: settings.reduceMotion,
      clickupConfigured: clickup.configured,
      workspaces: workspaces.liveItems,
      scratches: scratches.items,
      bangs: settings.bangs,
      threads: threads.agents,
      repos: repos.items,
      pullRequests: prs.items,
      runtimeFor: (id) => workspaceRecords.runtime(id),
      appControlsFor: appControls,
      recordFor: (id) => workspaceRecords.getRecord(id),
    },
    {
      openNewThread: () => app.openNewThread(),
      openScratch: (id) => app.openScratch(id),
      runBang: (id, extra) => void runBang(id, extra),
      openRepo: (id) => app.openRepo(id),
      reviewPullRequest,
      openSprint: () => app.openSprint(),
      openSettings: () => app.openSettings(),
      openWorkspace: (id, threadId) => app.openWorkspace(id, threadId),
      toggleReduceMotion: () => settings.toggleReduceMotion(),
      openSettingsSection,
      openWorkspaceThread,
      openWorkspaceFindings,
      controlApp: (id, action) => void controlApp(id, action),
      pullWorkspace: (id) => void pullWorkspace(id),
    },
  )
}
