import type { RepoRecord, WorkspaceRecord } from '$lib/ipc/bindings'
import { workspaceCardMetaText } from '$lib/homebase/card-status'
import { plural } from '$lib/sidebar/status'
import {
  isRunnableMacro,
  macroPromptPreview,
  type ScratchMacro,
} from '$lib/settings/scratch-macros'
import type { Scratch } from '$lib/state/scratches.svelte'
import type { Thread } from '$lib/state/threads.svelte'
import type { Workspace } from '$lib/state/workspaces.svelte'
import type { AppRunStatus } from '$lib/state/workspace-records.svelte'
import type { PaletteCommand } from './types'

export const ADD_TODO_COMMAND_ID = 'action-add-todo'

export type PaletteRegistryActions = {
  requestNewWorkspace: () => void
  requestNewScratch: () => void
  openScratch: (scratchId: string) => void
  runScratchMacro: (macroId: string, extra?: string) => void
  openHomebase: () => void
  openTodos: () => void
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
  scratches: Scratch[]
  scratchMacros: ScratchMacro[]
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
      id: 'action-new-scratch',
      group: 'Actions',
      label: 'New scratch',
      kbd: '⌘S',
      run: () => actions.requestNewScratch(),
    },
    {
      id: 'action-homebase',
      group: 'Actions',
      label: 'Go to Homebase',
      run: () => actions.openHomebase(),
    },
    {
      id: 'action-todos',
      group: 'Actions',
      label: 'Go to TODOs',
      run: () => actions.openTodos(),
    },
    {
      id: ADD_TODO_COMMAND_ID,
      group: 'Actions',
      label: 'Add a task',
      meta: 'todo',
      // The palette handles this one: it switches to todo mode instead of closing.
      run: () => {},
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
      label: 'Open Settings › Agents',
      run: () => actions.openSettingsSection('agents'),
    },
  ]

  for (const macro of state.scratchMacros) {
    if (!isRunnableMacro(macro)) continue
    cmds.push({
      id: `macro-run-${macro.id}`,
      group: 'Macros',
      label: macro.name.trim(),
      subtitle: macroPromptPreview(macro.prompt),
      run: () => actions.runScratchMacro(macro.id),
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
