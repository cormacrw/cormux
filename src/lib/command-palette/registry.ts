import type { RepoRecord } from '$lib/ipc/bindings'
import { workspaceCardMetaText } from './workspace-meta'
import { plural } from '$lib/format/plural'
import { shortcutLabel } from '$lib/keyboard/shortcuts'
import type { AppControls } from '$lib/run-app/controls'
import { appControls, controlApp } from '$lib/run-app/actions'
import { bangPreview, isUsableBang, type Bang } from '$lib/bangs/bangs'
import { navigate } from '$lib/core/nav.svelte'
import {
  openWorkspace,
  openWorkspaceFindings,
  openWorkspaceThread,
} from '$lib/nav/open'
import { updateSettings } from '$lib/settings/actions'
import { openSettings } from '$lib/settings/open'
import { settings } from '$lib/settings/store.svelte'
import { clickup } from '$lib/clickup/store.svelte'
import { findings } from '$lib/findings/store.svelte'
import { workspacePr } from '$lib/changes/workspace-pr'
import { attention } from '$lib/state/attention.svelte'
import { repos } from '$lib/state/repos.svelte'
import { scratches, type Scratch } from '$lib/state/scratches.svelte'
import { threads, type Thread } from '$lib/state/threads.svelte'
import { workspaces, type Workspace } from '$lib/state/workspaces.svelte'
import { pullWorkspace } from '$lib/workspace/git-actions'
import type { PaletteCommand } from './types'
import { runBang } from './actions'

/** What the palette lists, read from the stores; tests hand in their own. */
export type PaletteState = {
  reduceMotion: boolean
  /** The Sprint page only exists once a ClickUp API key is stored. */
  clickupConfigured: boolean
  workspaces: Workspace[]
  scratches: Scratch[]
  bangs: Bang[]
  /** Retired threads too: the palette is one of the two ways back into them. */
  threadsOf: (workspaceId: string) => Thread[]
  repos: RepoRecord[]
  /** The number of the PR on the workspace's branch, the one its sidebar row shows. */
  prNumberOf: (workspace: Workspace) => number | null
  appControlsFor: (workspaceId: string) => AppControls
  findingsAvailable: (workspace: Workspace) => boolean
  needsYou: (workspaceId: string) => boolean
}

function storeState(): PaletteState {
  return {
    reduceMotion: settings.values.reduceMotion,
    clickupConfigured: clickup.configured,
    workspaces: workspaces.liveItems,
    scratches: scratches.items,
    bangs: settings.values.bangs,
    threadsOf: (workspaceId) => threads.forWorkspace(workspaceId),
    repos: repos.items,
    prNumberOf: (workspace) => workspacePr(workspace)?.number ?? null,
    appControlsFor: appControls,
    findingsAvailable: (workspace) => findings.tabAvailable(workspace),
    needsYou: (workspaceId) => attention.needsYou(workspaceId),
  }
}

/** A PR goes first in the meta, so its number finds the thread (see `filterCommands`). */
function withPr(prNumber: number | null, meta: string): string {
  return prNumber == null ? meta : `PR #${prNumber} · ${meta}`
}

export function buildPaletteCommands(
  state: PaletteState = storeState(),
): PaletteCommand[] {
  const cmds: PaletteCommand[] = [
    {
      id: 'action-new-thread',
      group: 'Actions',
      label: 'New thread',
      kbd: shortcutLabel('newThread'),
      run: () => navigate({ view: 'newThread' }),
    },
    ...(state.clickupConfigured
      ? ([
          {
            id: 'action-sprint',
            group: 'Actions',
            label: 'Go to sprint',
            meta: 'clickup',
            icon: 'sprint',
            run: () => navigate({ view: 'sprint' }),
          },
        ] satisfies PaletteCommand[])
      : []),
    {
      id: 'action-settings',
      group: 'Actions',
      label: 'Open settings',
      icon: 'settings',
      run: () => openSettings(),
    },
    {
      id: 'action-reduce-motion',
      group: 'Actions',
      label: state.reduceMotion
        ? 'Disable reduced motion'
        : 'Enable reduced motion',
      icon: 'settings',
      run: () =>
        updateSettings({ reduceMotion: !settings.values.reduceMotion }),
    },
    {
      id: 'action-settings-repos',
      group: 'Actions',
      label: 'Open Settings › Repositories',
      icon: 'settings',
      run: () => openSettings({ section: 'repos' }),
    },
    {
      id: 'action-settings-engines',
      group: 'Actions',
      label: 'Open Settings › Agents',
      icon: 'settings',
      run: () => openSettings({ section: 'agents' }),
    },
  ]

  for (const bang of state.bangs) {
    if (!isUsableBang(bang)) continue
    cmds.push({
      id: `bang-run-${bang.id}`,
      kind: 'bang',
      group: 'Bangs',
      icon: 'bang',
      label: `!${bang.name}`,
      subtitle: bangPreview(bang.prompt),
      meta: 'Start a thread',
      run: () => void runBang(bang.id),
    })
  }

  for (const workspace of state.workspaces) {
    const wsThreads = state.threadsOf(workspace.id)
    cmds.push({
      id: `ws-open-${workspace.id}`,
      group: 'Workspaces',
      label: `Open ${workspace.name}`,
      meta: withPr(
        state.prNumberOf(workspace),
        workspaceCardMetaText(
          state.needsYou(workspace.id),
          wsThreads.filter((thread) => !thread.retired),
        ),
      ),
      icon: 'workspace',
      run: () => openWorkspace(workspace.id),
    })
  }

  for (const repo of state.repos) {
    cmds.push({
      id: `repo-open-${repo.id}`,
      group: 'Actions',
      label: `Open ${repo.name}`,
      meta: 'repo',
      run: () => navigate({ view: 'repo', repoId: repo.id }),
    })
  }

  for (const scratch of state.scratches) {
    cmds.push({
      id: `scratch-open-${scratch.id}`,
      group: 'Scratches',
      label: `Open ${scratch.title}`,
      meta: scratch.repoId,
      icon: 'thread',
      run: () => navigate({ view: 'scratch', scratchId: scratch.id }),
    })
  }

  for (const workspace of state.workspaces) {
    const wsThreads = state.threadsOf(workspace.id)
    // The workspace's own row opens its one thread; older ones are only here and in its header.
    for (const thread of wsThreads.filter((row) => row.retired)) {
      cmds.push({
        id: `thread-open-${thread.id}`,
        group: 'Threads',
        label: `Open ${thread.role} in ${workspace.name}`,
        meta: thread.activity,
        icon: 'thread',
        run: () => openWorkspaceThread(workspace.id, thread.id),
      })
    }
    if (state.findingsAvailable(workspace)) {
      cmds.push({
        id: `findings-${workspace.id}`,
        group: 'Threads',
        label: `Open Findings in ${workspace.name}`,
        icon: 'findings',
        run: () => openWorkspaceFindings(workspace.id),
      })
    }
  }

  for (const workspace of state.workspaces) {
    const controls = state.appControlsFor(workspace.id)
    const { start } = controls
    if (start?.enabled) {
      const restart = start.action === 'restart'
      cmds.push({
        id: `app-${start.action}-${workspace.id}`,
        group: 'App',
        label: `${restart ? 'Restart' : 'Run'} app in ${workspace.name}`,
        meta: controls.runCommand || undefined,
        icon: restart ? 'restart' : 'run',
        run: () => {
          openWorkspace(workspace.id)
          void controlApp(workspace.id, start.action)
        },
      })
    }
    if (controls.canRestart) {
      const { port } = workspace.runtime
      cmds.push({
        id: `app-restart-${workspace.id}`,
        group: 'App',
        label: `Restart app in ${workspace.name}`,
        meta: port ? `localhost:${port}` : undefined,
        icon: 'restart',
        run: () => {
          openWorkspace(workspace.id)
          void controlApp(workspace.id, 'restart')
        },
      })
    }
    if (controls.canStop) {
      cmds.push({
        id: `app-stop-${workspace.id}`,
        group: 'App',
        label: `Stop app in ${workspace.name}`,
        icon: 'stop',
        run: () => void controlApp(workspace.id, 'stop'),
      })
    }
  }

  for (const workspace of state.workspaces) {
    const { behind } = workspace.runtime
    if (behind <= 0) continue
    cmds.push({
      id: `git-pull-${workspace.id}`,
      group: 'Git',
      label: `Pull ${plural(behind, 'commit')} from ${workspace.base} into ${workspace.name}`,
      icon: 'pull',
      run: () => void pullWorkspace(workspace.id),
    })
  }

  return cmds
}
