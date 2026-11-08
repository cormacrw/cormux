import type { RepoRecord, WorkspaceAppControlAction } from '$lib/ipc/bindings'
import { workspaceCardMetaText } from './workspace-meta'
import { plural } from '$lib/format/plural'
import { shortcutLabel } from '$lib/keyboard/shortcuts'
import type { AppControls } from '$lib/run-app/controls'
import { bangPreview, isUsableBang, type Bang } from '$lib/bangs/bangs'
import type { PullRequest } from '$lib/state/prs.svelte'
import type { Scratch } from '$lib/state/scratches.svelte'
import type { Thread } from '$lib/state/threads.svelte'
import type { Workspace } from '$lib/state/workspaces.svelte'
import type { SettingsSectionId } from '$lib/core/route'
import type { PaletteCommand } from './types'

type PaletteRegistryActions = {
  openNewThread: () => void
  openScratch: (scratchId: string) => void
  runBang: (bangId: string) => void
  openRepo: (repoId: string) => void
  reviewPullRequest: (pr: PullRequest) => void
  openSprint: () => void
  openSettings: () => void
  openWorkspace: (workspaceId: string, threadId?: string) => void
  toggleReduceMotion: () => void
  openSettingsSection: (section: SettingsSectionId) => void
  openWorkspaceThread: (workspaceId: string, threadId: string) => void
  openWorkspaceFindings: (workspaceId: string) => void
  controlApp: (workspaceId: string, action: WorkspaceAppControlAction) => void
  pullWorkspace: (workspaceId: string) => void
}

type PaletteRegistryState = {
  reduceMotion: boolean
  /** The Sprint page only exists once a ClickUp API key is stored. */
  clickupConfigured: boolean
  workspaces: Workspace[]
  scratches: Scratch[]
  bangs: Bang[]
  /** Retired threads too: the palette is one of the two ways back into them. */
  threads: Thread[]
  repos: RepoRecord[]
  pullRequests: PullRequest[]
  appControlsFor: (workspaceId: string) => AppControls
}

export function buildPaletteCommandsFromState(
  state: PaletteRegistryState,
  actions: PaletteRegistryActions,
): PaletteCommand[] {
  const cmds: PaletteCommand[] = [
    {
      id: 'action-new-thread',
      group: 'Actions',
      label: 'New thread',
      kbd: shortcutLabel('newThread'),
      run: () => actions.openNewThread(),
    },
    ...(state.clickupConfigured
      ? ([
          {
            id: 'action-sprint',
            group: 'Actions',
            label: 'Go to sprint',
            meta: 'clickup',
            icon: 'sprint',
            run: () => actions.openSprint(),
          },
        ] satisfies PaletteCommand[])
      : []),
    {
      id: 'action-settings',
      group: 'Actions',
      label: 'Open settings',
      icon: 'settings',
      run: () => actions.openSettings(),
    },
    {
      id: 'action-reduce-motion',
      group: 'Actions',
      label: state.reduceMotion
        ? 'Disable reduced motion'
        : 'Enable reduced motion',
      icon: 'settings',
      run: () => actions.toggleReduceMotion(),
    },
    {
      id: 'action-settings-repos',
      group: 'Actions',
      label: 'Open Settings › Repositories',
      icon: 'settings',
      run: () => actions.openSettingsSection('repos'),
    },
    {
      id: 'action-settings-engines',
      group: 'Actions',
      label: 'Open Settings › Agents',
      icon: 'settings',
      run: () => actions.openSettingsSection('agents'),
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
      run: () => actions.runBang(bang.id),
    })
  }

  const threadsByWorkspace = new Map<string, Thread[]>()
  for (const thread of state.threads) {
    const list = threadsByWorkspace.get(thread.workspaceId)
    if (list) list.push(thread)
    else threadsByWorkspace.set(thread.workspaceId, [thread])
  }

  for (const workspace of state.workspaces) {
    const wsThreads = threadsByWorkspace.get(workspace.id) ?? []
    cmds.push({
      id: `ws-open-${workspace.id}`,
      group: 'Workspaces',
      label: `Open ${workspace.name}`,
      meta: workspaceCardMetaText(
        workspace,
        wsThreads.filter((thread) => !thread.retired),
      ),
      icon: 'workspace',
      run: () => actions.openWorkspace(workspace.id),
    })
  }

  for (const repo of state.repos) {
    cmds.push({
      id: `repo-open-${repo.id}`,
      group: 'Actions',
      label: `Open ${repo.name}`,
      meta: 'repo',
      run: () => actions.openRepo(repo.id),
    })
  }

  for (const pr of state.pullRequests) {
    cmds.push({
      id: `pr-review-${pr.id}`,
      group: 'Actions',
      label: `Review PR #${pr.num} ${pr.title}`,
      meta: 'pull request',
      run: () => actions.reviewPullRequest(pr),
    })
  }

  for (const scratch of state.scratches) {
    cmds.push({
      id: `scratch-open-${scratch.id}`,
      group: 'Scratches',
      label: `Open ${scratch.title}`,
      meta: scratch.repoId,
      icon: 'thread',
      run: () => actions.openScratch(scratch.id),
    })
  }

  for (const workspace of state.workspaces) {
    const wsThreads = threadsByWorkspace.get(workspace.id) ?? []
    // The workspace's own row opens its one thread; older ones are only here and in its header.
    for (const thread of wsThreads.filter((row) => row.retired)) {
      cmds.push({
        id: `thread-open-${thread.id}`,
        group: 'Threads',
        label: `Open ${thread.role} in ${workspace.name}`,
        meta: thread.activity,
        icon: 'thread',
        run: () => actions.openWorkspaceThread(workspace.id, thread.id),
      })
    }
    cmds.push({
      id: `findings-${workspace.id}`,
      group: 'Threads',
      label: `Open Findings in ${workspace.name}`,
      icon: 'findings',
      run: () => actions.openWorkspaceFindings(workspace.id),
    })
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
          actions.openWorkspace(workspace.id)
          actions.controlApp(workspace.id, start.action)
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
          actions.openWorkspace(workspace.id)
          actions.controlApp(workspace.id, 'restart')
        },
      })
    }
    if (controls.canStop) {
      cmds.push({
        id: `app-stop-${workspace.id}`,
        group: 'App',
        label: `Stop app in ${workspace.name}`,
        icon: 'stop',
        run: () => actions.controlApp(workspace.id, 'stop'),
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
      run: () => actions.pullWorkspace(workspace.id),
    })
  }

  return cmds
}
