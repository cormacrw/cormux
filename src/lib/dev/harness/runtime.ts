import {
  fixtureDiff,
  fixtureSnapshot,
  fixtureTimeline,
} from '../fixture-snapshot'
import type { HarnessControls, HarnessScenario } from './scenario'
import { nextSeq } from './seed'

type Callback = (...args: unknown[]) => void

export type HarnessChannel = { id: number; index: number }

/** What the handlers share: the scenario, and Tauri's callback, channel and event plumbing. */
export type HarnessRuntime = ReturnType<typeof createRuntime>

export function createRuntime(scenario: HarnessScenario) {
  const callbacks = new Map<number, Callback>()
  const eventListeners = new Map<string, number[]>()
  const agentChannels = new Map<string, HarnessChannel>()
  let nextCallbackId = 1
  let nextEventId = 1

  const newId = () => nextEventId++

  const send = (channel: HarnessChannel, message: unknown) => {
    callbacks.get(channel.id)?.({ index: channel.index++, message })
  }

  const emitStateChanged = () => {
    for (const handler of eventListeners.get('state-changed') ?? []) {
      callbacks.get(handler)?.({
        event: 'state-changed',
        id: newId(),
        payload: {
          version: nextEventId,
          change: { threads: true, repoGit: false },
        },
      })
    }
  }

  const controls: HarnessControls = {
    prompts: [],
    resolved: [],
    answered: [],
    opened: [],
    openedTerminal: null,
    resumed: null,
    clickupCalls: [],
    stackRemoteUp: false,
    setStatus(name, status) {
      for (const row of fixtureSnapshot.workspaces) {
        if (row.name === name) row.status = status as typeof row.status
      }
      emitStateChanged()
    },
    setThreadStatus(threadId, status) {
      for (const row of fixtureSnapshot.persisted.threads) {
        if (row.id === threadId) row.status = status as typeof row.status
      }
      emitStateChanged()
    },
    requestApproval(threadId) {
      const approvals = fixtureSnapshot.persisted.approvals
      approvals.push({
        id: `approval-${approvals.length + 1}`,
        threadId,
        status: 'pending',
        tool: 'execute',
        payload: {
          title: 'Run a command',
          what: 'pnpm test',
          why: 'Bash',
          okLabel: 'Approve',
          noLabel: 'Deny',
        },
      })
      emitStateChanged()
    },
    agentEvents(events) {
      for (const event of events) {
        const seq = nextSeq('th-lead')
        const { type } = event as { type: string }
        const kind = type === 'toolCall' || type === 'step' ? 'tool' : 'message'
        fixtureTimeline.push({
          id: 950_000 + seq,
          threadId: 'th-lead',
          seq,
          kind,
          payload: JSON.stringify(event),
          createdAt: new Date()
            .toISOString()
            .replace('T', ' ')
            .replace('Z', ''),
        })
        const channel = agentChannels.get('th-lead')
        if (channel) send(channel, { seq, event })
      }
    },
    commit() {
      if (!runtime.diffChannel) return
      send(runtime.diffChannel, {
        workspaceId: fixtureDiff.workspaceId,
        diff: {
          ...fixtureDiff,
          files: [],
          uncommitted: { added: 0, deleted: 0 },
        },
      })
    },
  }

  const runtime = {
    scenario,
    newId,
    send,
    emitStateChanged,
    agentChannels,
    /** Changes' subscription to the fixture workspace, once it has one. */
    diffChannel: null as HarnessChannel | null,
    controls,
    /** Per-thread picks, which `create_workspace` also sets. */
    threadModels: new Map<string, string | null>(),
    threadEfforts: new Map<string, string | null>(),
    listen(event: string, handler: number) {
      eventListeners.set(event, [...(eventListeners.get(event) ?? []), handler])
    },
    transformCallback(callback?: Callback, once = false) {
      const id = nextCallbackId++
      callbacks.set(id, (...args: unknown[]) => {
        if (once) callbacks.delete(id)
        callback?.(...args)
      })
      return id
    },
    unregisterCallback(id: number) {
      callbacks.delete(id)
    },
  }
  return runtime
}
