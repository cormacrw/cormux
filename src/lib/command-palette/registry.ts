import { updateSettings } from '$lib/settings/actions'
import { openWorkspace } from '$lib/nav/open'
import { navigate } from '$lib/core/nav.svelte'
import { reviewPullRequest } from '$lib/review/start-review'
import { appControls, controlApp } from '$lib/run-app/actions'
import { openSettings } from '$lib/settings/open'
import { clickup } from '$lib/clickup/store.svelte'
import { repos } from '$lib/state/repos.svelte'
import { scratches } from '$lib/state/scratches.svelte'
import { settings } from '$lib/settings/store.svelte'
import { prs } from '$lib/state/prs.svelte'
import { threads } from '$lib/state/threads.svelte'
import { findings } from '$lib/findings/store.svelte'
import { workspaces } from '$lib/state/workspaces.svelte'
import { pullWorkspace } from '$lib/workspace/git-actions'
import { openWorkspaceFindings, openWorkspaceThread } from '$lib/nav/open'
import type { PaletteCommand } from './types'
import { buildPaletteCommandsFromState } from './build-registry'
import { runBang } from './actions'

export function buildPaletteCommands(): PaletteCommand[] {
  return buildPaletteCommandsFromState(
    {
      reduceMotion: settings.values.reduceMotion,
      clickupConfigured: clickup.configured,
      workspaces: workspaces.liveItems,
      scratches: scratches.items,
      bangs: settings.values.bangs,
      threads: threads.items,
      repos: repos.items,
      pullRequests: prs.items,
      appControlsFor: appControls,
      findingsAvailable: (workspace) => findings.tabAvailable(workspace),
    },
    {
      openNewThread: () => navigate({ view: 'newThread' }),
      openScratch: (id) => navigate({ view: 'scratch', scratchId: id }),
      runBang: (id) => void runBang(id),
      openRepo: (id) => navigate({ view: 'repo', repoId: id }),
      reviewPullRequest,
      openSprint: () => navigate({ view: 'sprint' }),
      openSettings: () => openSettings(),
      openWorkspace: (id, threadId) => openWorkspace(id, { threadId }),
      toggleReduceMotion: () =>
        updateSettings({ reduceMotion: !settings.values.reduceMotion }),
      openSettingsSection: (section) => openSettings({ section }),
      openWorkspaceThread,
      openWorkspaceFindings,
      controlApp: (id, action) => void controlApp(id, action),
      pullWorkspace: (id) => void pullWorkspace(id),
    },
  )
}
