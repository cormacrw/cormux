import type { RepoRecord, WorkspaceRecord } from '$lib/ipc/bindings'
import { workspaceCardMetaText } from '$lib/homebase/card-status'
import { plural } from '$lib/sidebar/status'
import type { Thread } from '$lib/state/threads.svelte'
import type { Workspace } from '$lib/state/workspaces.svelte'
import type { AppRunStatus } from '$lib/state/workspace-records.svelte'
import type { PaletteCommand } from './types'

export type PaletteRegistryActions = {
  requestNewWorkspace: () => void
  openHomebase: () => void
  openSettings: () => void
  openWorkspace: (workspaceId: string, threadId?: string) => void
  toggleReduceMotion: () => void
  openSettingsSection: (section: string) => void
  openWorkspaceThread: (workspaceId: string, threadId: string) => void
  openWorkspaceFindings: (workspaceId: string) => void
  requestNewThread: (workspaceId: string) => void
  runWorkspaceApp: (
    workspaceId: string,
    action: 'run' | 'restart' | 'stop',
  ) => void
  pullWorkspace: (workspaceId: string) => void
}

export type PaletteRegistryState = {
  reduceMotion: boolean
  workspaces: Workspace[]
  threads: Thread[]
  repos: RepoRecord[]
  records: WorkspaceRecord[]
  runtimeFor: (workspaceId: string) => {
    appStatus: AppRunStatus
    port: number | null
    behind: number
  }
  repoFor: (repoId: string) => RepoRecord | undefined
  recordFor: (workspaceId: string) => WorkspaceRecord | undefined
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
      id: 'action-new-workspace',
      group: 'Actions',
      label: 'New workspace',
      kbd: '⌘N',
      run: () => actions.requestNewWorkspace(),
    },
    {
      id: 'action-homebase',
      group: 'Actions',
      label: 'Go to Homebase',
      run: () => actions.openHomebase(),
    },
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
      label: 'Open Settings › Engines',
      run: () => actions.openSettingsSection('engines'),
    },
  ]

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

  for (const workspace of state.workspaces) {
    const wsThreads = state.threads.filter(
      (t) => t.workspaceId === workspace.id,
    )
    for (const thread of wsThreads) {
      cmds.push({
        id: `thread-open-${thread.id}`,
        group: 'Threads',
        label: `Open ${thread.role} in ${workspace.name}`,
        meta: thread.activity,
        run: () => actions.openWorkspaceThread(workspace.id, thread.id),
      })
    }
    cmds.push({
      id: `thread-new-${workspace.id}`,
      group: 'Threads',
      label: `New thread in ${workspace.name}`,
      run: () => actions.requestNewThread(workspace.id),
    })
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

    if (appStatus === 'stopped') {
      cmds.push({
        id: `app-run-${workspace.id}`,
        group: 'App',
        label: `Run app in ${workspace.name}`,
        meta: repo?.runCommand ?? 'No run command',
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
        meta: repo?.runCommand ?? 'No run command',
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
