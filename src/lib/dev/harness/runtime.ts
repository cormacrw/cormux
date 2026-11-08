import type { Change, TranscriptEvent } from '$lib/ipc/bindings'
import {
  appendEvent,
  currentConversation,
  findThread,
  findWorkspace,
  type HarnessDb,
} from './db'
import { createFixtures, LEAD_THREAD_ID } from './fixtures'
import type { HarnessControls, HarnessScenario } from './scenario'
import { seedFixtures } from './seed'

type Callback = (...args: unknown[]) => void

export type HarnessChannel = { id: number; index: number }

/** The `Change` flags the core sends, by what moved. */
export const CHANGE = {
  /** Anything the snapshot carries. */
  state: { repoGit: false },
  /** `origin` moved too, so the repos' behind and ahead counts are stale. */
  repoGit: { repoGit: true },
} satisfies Record<string, Change>

/**
 * One browser session's core: its data, the scenario it started from, and Tauri's callback,
 * channel and event plumbing.
 */
export type HarnessRuntime = ReturnType<typeof createRuntime>

export function createRuntime(scenario: HarnessScenario) {
  const db: HarnessDb = createFixtures()
  seedFixtures(db, scenario)
  const callbacks = new Map<number, Callback>()
  /** Per event, each listener's id and the callback it calls. */
  const eventListeners = new Map<string, Map<number, number>>()
  const agentChannels = new Map<string, HarnessChannel>()
  let nextCallbackId = 1
  let nextId = 1

  const newId = () => nextId++

  const send = (channel: HarnessChannel, message: unknown) => {
    callbacks.get(channel.id)?.({ index: channel.index++, message })
  }

  /** Saves an event in a thread's transcript, then streams it with its `seq`. */
  const append = (threadId: string, event: TranscriptEvent) => {
    const { seq } = appendEvent(db, threadId, event)
    const channel = agentChannels.get(threadId)
    if (channel) {
      send(channel, {
        seq,
        conversation: currentConversation(db, threadId),
        event,
      })
    }
    return seq
  }

  /** Like the core's notifier: numbered one after the snapshot's version, which it moves. */
  const emitStateChanged = (change: Change) => {
    db.snapshot.version += 1
    for (const handler of eventListeners.get('state-changed')?.values() ?? []) {
      callbacks.get(handler)?.({
        event: 'state-changed',
        id: newId(),
        payload: { version: db.snapshot.version, change },
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
      for (const record of db.snapshot.workspaces) {
        if (record.name === name) record.status = status
      }
      emitStateChanged(CHANGE.state)
    },
    setThreadStatus(threadId, status) {
      const thread = findThread(db, threadId)
      if (thread) thread.status = status
      emitStateChanged(CHANGE.state)
    },
    requestApproval(threadId) {
      const approvals = db.snapshot.persisted.approvals
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
      emitStateChanged(CHANGE.state)
    },
    agentEvents(events) {
      for (const event of events) {
        append(LEAD_THREAD_ID, event as TranscriptEvent)
      }
    },
    commit() {
      if (!runtime.diffChannel) return
      send(runtime.diffChannel, {
        workspaceId: db.diff.workspaceId,
        diff: {
          ...db.diff,
          files: [],
          uncommitted: { added: 0, deleted: 0 },
        },
      })
    },
  }

  const runtime = {
    db,
    scenario,
    newId,
    send,
    append,
    emitStateChanged,
    findThread: (threadId: string) => findThread(db, threadId),
    findWorkspace: (workspaceId: string) => findWorkspace(db, workspaceId),
    agentChannels,
    /** Changes' subscription to the fixture workspace, once it has one. */
    diffChannel: null as HarnessChannel | null,
    controls,
    /** Per-thread picks, which `create_workspace` also sets. */
    threadModels: new Map<string, string | null>(),
    threadEfforts: new Map<string, string | null>(),
    listen(event: string, handler: number) {
      const id = newId()
      const listeners = eventListeners.get(event) ?? new Map<number, number>()
      eventListeners.set(event, listeners.set(id, handler))
      return id
    },
    unlisten(event: string, id: number) {
      eventListeners.get(event)?.delete(id)
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
