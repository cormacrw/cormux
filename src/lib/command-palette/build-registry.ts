import type { RepoRecord, WorkspaceRecord } from '$lib/ipc/bindings'
import { workspaceCardMetaText } from '$lib/homebase/card-status'
import { plural } from '$lib/sidebar/status'
import { bangPreview, isUsableBang, type Bang } from '$lib/settings/bangs'
import type { PullRequest } from '$lib/state/prs.svelte'
import type { Scratch } from '$lib/state/scratches.svelte'
import type { Thread } from '$lib/state/threads.svelte'
import type { Workspace } from '$lib/state/workspaces.svelte'
import type { AppRunStatus } from '$lib/state/workspace-records.svelte'
import type { PaletteCommand } from './types'

export type PaletteRegistryActions = {
  openNewThread: () => void
  openScratch: (scratchId: string) => void
  runBang: (bangId: string, extra?: string) => void
  openRepo: (repoId: string) => void
  reviewPullRequest: (pr: PullRequest) => void
  openSprint: () => void
  openSettings: () => void
  openWorkspace: (workspaceId: string, threadId?: string) => void
  toggleReduceMotion: () => void
  openSettingsSection: (section: string) => void
  openWorkspaceThread: (workspaceId: string, threadId: string) => void
  openWorkspaceFindings: (workspaceId: string) => void
  runWorkspaceApp: (
    workspaceId: string,
    action: 'run' | 'restart' | 'stop',
  ) => void
  pullWorkspace: (workspaceId: string) => void
}

export type PaletteRegistryState = {
  reduceMotion: boolean
  /** The Sprint page only exists once a ClickUp API key is stored. */
  clickupConfigured: boolean
  workspaces: Workspace[]
  scratches: Scratch[]
  bangs: Bang[]
  threads: Thread[]
  repos: RepoRecord[]
  pullRequests: PullRequest[]
  records: WorkspaceRecord[]
  runtimeFor: (workspaceId: string) => {
    appStatus: AppRunStatus
    port: number | null
    behind: number
  }
  repoFor: (repoId: string) => RepoRecord | undefined
  recordFor: (workspaceId: string) => WorkspaceRecord | undefined
  /** The command Run starts in a workspace, its own override included; '' for none. */
  runCommandFor?: (workspaceId: string) => string
}

function isProvisioning(lifecycle: string) {
  return lifecycle === 'creating' || lifecycle === 'provisioning'
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
      kbd: '⌘N',
      run: () => actions.openNewThread(),
    },
    ...(state.clickupConfigured
      ? [
          {
            id: 'action-sprint',
            group: 'Actions',
            label: 'Go to sprint',
            meta: 'clickup',
            run: () => actions.openSprint(),
          },
        ]
      : []),
    {
      id: 'action-settings',
      group: 'Actions',
      label: 'Open settings',
      run: () => actions.openSettings(),
    },
    {
      id: 'action-reduce-motion',
      group: 'Actions',
      label: state.reduceMotion
        ? 'Disable reduced motion'
        : 'Enable reduced motion',
      run: () => actions.toggleReduceMotion(),
    },
    {
      id: 'action-settings-repos',
      group: 'Actions',
      label: 'Open Settings › Repositories',
      run: () => actions.openSettingsSection('repos'),
    },
    {
      id: 'action-settings-engines',
      group: 'Actions',
      label: 'Open Settings › Agents',
      run: () => actions.openSettingsSection('agents'),
    },
  ]

  for (const bang of state.bangs) {
    if (!isUsableBang(bang)) continue
    cmds.push({
      id: `bang-run-${bang.id}`,
      group: 'Bangs',
      label: `!${bang.name}`,
      subtitle: bangPreview(bang.prompt),
      meta: 'Start a scratch',
      run: () => actions.runBang(bang.id),
    })
  }

  for (const workspace of state.workspaces) {
    const wsThreads = state.threads.filter(
      (t) => t.workspaceId === workspace.id,
    )
    cmds.push({
      id: `ws-open-${workspace.id}`,
      group: 'Workspaces',
      label: `Open ${workspace.name}`,
      meta: workspaceCardMetaText(workspace, wsThreads),
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

  // After Workspaces, before App.
  for (const scratch of state.scratches) {
    cmds.push({
      id: `scratch-open-${scratch.id}`,
      group: 'Scratches',
      label: `Open ${scratch.title}`,
      meta: scratch.repoId,
      run: () => actions.openScratch(scratch.id),
    })
  }

  for (const workspace of state.workspaces) {
    const wsThreads = state.threads.filter(
      (t) => t.workspaceId === workspace.id,
    )
    // The workspace's own row opens its one thread; older ones are only here and in its header.
    for (const thread of wsThreads.filter((row) => row.retired)) {
      cmds.push({
        id: `thread-open-${thread.id}`,
        group: 'Threads',
        label: `Open ${thread.role} in ${workspace.name}`,
        meta: thread.activity,
        run: () => actions.openWorkspaceThread(workspace.id, thread.id),
      })
    }
    cmds.push({
      id: `findings-${workspace.id}`,
      group: 'Threads',
      label: `Open Findings in ${workspace.name}`,
      run: () => actions.openWorkspaceFindings(workspace.id),
    })
  }

  for (const workspace of state.workspaces) {
    if (isProvisioning(workspace.lifecycle)) continue
    const record = state.recordFor(workspace.id)
    const repo = record ? state.repoFor(record.repoId) : undefined
    const runtime = state.runtimeFor(workspace.id)
    const { appStatus } = runtime
    const runCommand =
      state.runCommandFor?.(workspace.id) ?? repo?.runCommand ?? ''

    if (appStatus === 'stopped') {
      cmds.push({
        id: `app-run-${workspace.id}`,
        group: 'App',
        label: `Run app in ${workspace.name}`,
        meta: runCommand || 'No run command',
        run: () => {
          actions.openWorkspace(workspace.id)
          actions.runWorkspaceApp(workspace.id, 'run')
        },
      })
      continue
    }

    if (appStatus === 'crashed') {
      cmds.push({
        id: `app-restart-${workspace.id}`,
        group: 'App',
        label: `Restart app in ${workspace.name}`,
        meta: runCommand || 'No run command',
        run: () => {
          actions.openWorkspace(workspace.id)
          actions.runWorkspaceApp(workspace.id, 'restart')
        },
      })
      continue
    }

    if (appStatus === 'running') {
      cmds.push({
        id: `app-restart-${workspace.id}`,
        group: 'App',
        label: `Restart app in ${workspace.name}`,
        meta: runtime.port ? `localhost:${runtime.port}` : undefined,
        run: () => {
          actions.openWorkspace(workspace.id)
          actions.runWorkspaceApp(workspace.id, 'restart')
        },
      })
    }
    if (appStatus === 'starting' || appStatus === 'running') {
      cmds.push({
        id: `app-stop-${workspace.id}`,
        group: 'App',
        label: `Stop app in ${workspace.name}`,
        run: () => actions.runWorkspaceApp(workspace.id, 'stop'),
      })
    }
  }

  for (const workspace of state.workspaces) {
    const record = state.recordFor(workspace.id)
    if (!record) continue
    const behind = state.runtimeFor(workspace.id).behind
    if (behind <= 0) continue
    cmds.push({
      id: `git-pull-${workspace.id}`,
      group: 'Git',
      label: `Pull ${plural(behind, 'commit')} from ${record.base} into ${workspace.name}`,
      run: () => actions.pullWorkspace(workspace.id),
    })
  }

  return cmds
}
