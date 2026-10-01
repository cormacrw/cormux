import Zap from '@lucide/svelte/icons/zap'
import Download from '@lucide/svelte/icons/download'
import GitBranch from '@lucide/svelte/icons/git-branch'
import Layers from '@lucide/svelte/icons/layers'
import ListTodo from '@lucide/svelte/icons/list-todo'
import MessageSquare from '@lucide/svelte/icons/message-square'
import Play from '@lucide/svelte/icons/play'
import Plus from '@lucide/svelte/icons/plus'
import RotateCw from '@lucide/svelte/icons/rotate-cw'
import Search from '@lucide/svelte/icons/search'
import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
import Square from '@lucide/svelte/icons/square'
import { app } from '$lib/state/app.svelte'
import { repos } from '$lib/state/repos.svelte'
import { scratches } from '$lib/state/scratches.svelte'
import { settings } from '$lib/state/settings.svelte'
import { threads } from '$lib/state/threads.svelte'
import { workspaceRecords } from '$lib/state/workspace-records.svelte'
import { workspaces } from '$lib/state/workspaces.svelte'
import type { PaletteCommand } from './types'
import { buildPaletteCommandsFromState } from './build-registry'
import {
  openSettingsSection,
  openWorkspaceFindings,
  openWorkspaceThread,
  pullWorkspace,
  requestNewThread,
  runScratchMacro,
  runWorkspaceApp,
  toggleReduceMotion,
} from './actions'

const iconByCommandPrefix: Record<string, PaletteCommand['icon']> = {
  'action-new-workspace': Plus,
  'action-new-scratch': Plus,
  'action-homebase': Layers,
  'action-todos': ListTodo,
  'action-add-todo': Plus,
  'action-settings': SlidersHorizontal,
  'action-reduce-motion': SlidersHorizontal,
  'action-settings-repos': SlidersHorizontal,
  'action-settings-engines': SlidersHorizontal,
}

const workspaceIcon = GitBranch
const threadIcons = {
  open: MessageSquare,
  new: Plus,
  findings: Search,
} as const
const appIcons = {
  run: Play,
  restart: RotateCw,
  stop: Square,
} as const

function attachIcons(commands: PaletteCommand[]): PaletteCommand[] {
  return commands.map((command) => {
    if (iconByCommandPrefix[command.id]) {
      return { ...command, icon: iconByCommandPrefix[command.id] }
    }
    if (command.id.startsWith('ws-open-')) {
      return { ...command, icon: workspaceIcon }
    }
    if (command.id.startsWith('macro-run-')) {
      return { ...command, icon: Zap }
    }
    if (command.id.startsWith('scratch-open-')) {
      return { ...command, icon: threadIcons.open }
    }
    if (command.id.startsWith('thread-open-')) {
      return { ...command, icon: threadIcons.open }
    }
    if (command.id.startsWith('thread-new-')) {
      return { ...command, icon: threadIcons.new }
    }
    if (command.id.startsWith('findings-')) {
      return { ...command, icon: threadIcons.findings }
    }
    if (command.id.includes('app-run-')) {
      return { ...command, icon: appIcons.run }
    }
    if (command.id.includes('app-restart-')) {
      return { ...command, icon: appIcons.restart }
    }
    if (command.id.includes('app-stop-')) {
      return { ...command, icon: appIcons.stop }
    }
    if (command.id.startsWith('git-pull-')) {
      return { ...command, icon: Download }
    }
    return command
  })
}

export function buildPaletteCommands(): PaletteCommand[] {
  const commands = buildPaletteCommandsFromState(
    {
      reduceMotion: settings.reduceMotion,
      workspaces: workspaces.liveItems,
      scratches: scratches.items,
      scratchMacros: settings.scratchMacros,
      threads: threads.agents,
      repos: repos.items,
      records: workspaceRecords.records,
      runtimeFor: (id) => workspaceRecords.runtime(id),
      repoFor: (id) => repos.getById(id),
      recordFor: (id) => workspaceRecords.getRecord(id),
    },
    {
      requestNewWorkspace: () => app.requestNewWorkspace(),
      requestNewScratch: () => app.requestNewScratch(),
      openScratch: (id) => app.openScratch(id),
      runScratchMacro: (id, extra) => void runScratchMacro(id, extra),
      openHomebase: () => app.openHomebase(),
      openTodos: () => app.openTodos(),
      openSettings: () => app.openSettings(),
      openWorkspace: (id, threadId) => app.openWorkspace(id, threadId),
      toggleReduceMotion,
      openSettingsSection,
      openWorkspaceThread,
      openWorkspaceFindings,
      requestNewThread,
      runWorkspaceApp,
      pullWorkspace,
    },
  )
  return attachIcons(commands)
}
