import { fixtureDiff, fixtureEngines, fixtureSnapshot } from './fixture-snapshot'

type InvokeArgs = Record<string, unknown> | undefined

function isRealTauri(): boolean {
  const internals = (
    window as Window & {
      __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label: string } } }
    }
  ).__TAURI_INTERNALS__
  return internals?.metadata?.currentWindow?.label != null
}

/**
 * Stubs Tauri IPC so `pnpm dev` and Playwright can drive the UI in Chromium.
 * No-ops when the real webview already injected internals.
 */
type ScratchInput = { title: string; repoId: string; prompt: string | null }

let nextScratch = 1

/** Mirrors `scratch::create`: the page opens with the prompt and an `Opened <repo>` row. */
function createScratch(input: ScratchInput) {
  const n = nextScratch++
  const scratchId = `scratch-harness-${n}`
  const threadId = `th-scratch-harness-${n}`
  const persisted = fixtureSnapshot.persisted
  const createdAt = new Date().toISOString().slice(0, 19).replace('T', ' ')
  const prompt = input.prompt?.trim()
  persisted.scratches = [
    {
      id: scratchId,
      repoId: input.repoId,
      title: input.title,
      threadId,
      engine: 'cursor',
      status: prompt ? 'running' : 'idle',
      createdAt,
    },
    ...persisted.scratches,
  ]
  if (prompt) {
    const repo = persisted.repos.find((row) => row.id === input.repoId)
    const events = [
      { type: 'messageChunk', role: 'user', text: prompt },
      {
        type: 'toolCall',
        id: `open-${n}`,
        title: `Opened ${input.repoId}`,
        name: null,
        kind: 'read',
        status: 'completed',
        locations: [],
        detail: repo?.path ?? input.repoId,
      },
    ]
    events.forEach((event, index) => {
      persisted.timeline.push({
        id: 1000 * n + index,
        threadId,
        seq: index + 1,
        kind: event.type === 'toolCall' ? 'tool' : 'message',
        payload: JSON.stringify(event),
        createdAt,
      })
    })
  }
  return { scratchId, threadId }
}

/** `?longThread=N` fills the Lead thread with N finished turns (virtualized timeline). */
function seedLongThread(turns: number) {
  const timeline = fixtureSnapshot.persisted.timeline
  let id = 100_000
  let seq = 0
  const push = (kind: string, event: object) => {
    seq += 1
    id += 1
    timeline.push({
      id,
      threadId: 'th-lead',
      seq,
      kind,
      payload: JSON.stringify(event),
      createdAt: '2026-09-28 12:00:00',
    })
  }
  for (let turn = 1; turn <= turns; turn += 1) {
    push('message', { type: 'messageChunk', role: 'user', text: `Question ${turn}` })
    push('message', { type: 'messageChunk', role: 'agent', text: `Answer ${turn}` })
    push('turn_end', { type: 'turnEnd', stop_reason: 'end_turn', error: null })
  }
  // Switching away from a branch and back writes two notes with the same title.
  for (const branch of ['feat/test', 'feat/colors', 'feat/test']) {
    push('tool', { icon: 'branch', title: `Switched to \`${branch}\``, detail: 'Checked out in this worktree' })
  }
}

export function installBrowserHarness() {
  if (typeof window === 'undefined' || isRealTauri()) return
  const longThread = Number(new URLSearchParams(window.location.search).get('longThread'))
  if (longThread > 0) seedLongThread(longThread)
  // Tests can inject a real thread's events (as exported from cormux.db) for the Lead thread.
  const injected = (window as { __HARNESS_TIMELINE__?: typeof fixtureSnapshot.persisted.timeline })
    .__HARNESS_TIMELINE__
  if (injected) {
    const persisted = fixtureSnapshot.persisted
    persisted.timeline = [...persisted.timeline.filter((row) => row.threadId !== 'th-lead'), ...injected]
  }

  const callbacks = new Map<number, (...args: unknown[]) => void>()
  let nextCallbackId = 1
  let nextEventId = 1
  const agentChannels = new Map<string, { id: number; index: number }>()
  const eventListeners = new Map<string, number[]>()

  const emitStateChanged = () => {
    for (const handler of eventListeners.get('state-changed') ?? []) {
      callbacks.get(handler)?.({
        event: 'state-changed',
        id: nextEventId++,
        payload: { version: nextEventId, kind: 'workspaceStatus' },
      })
    }
  }

  // Stream a reply the way a real engine does: many small chunks, then a turn end that
  // the backend follows with a state-changed refetch.
  const streamReply = (threadId: string, text: string) => {
    const timeline = fixtureSnapshot.persisted.timeline
    let seq = Math.max(0, ...timeline.filter((row) => row.threadId === threadId).map((row) => row.seq))
    const persist = (kind: string, event: object) => {
      seq += 1
      timeline.push({ id: 900_000 + seq, threadId, seq, kind, payload: JSON.stringify(event), createdAt: '2026-09-28 12:00:00' })
    }
    const send = (event: object) => {
      const channel = agentChannels.get(threadId)
      if (!channel) return
      callbacks.get(channel.id)?.({ index: channel.index++, message: event })
    }
    persist('message', { type: 'messageChunk', role: 'user', text })
    const words = `Reply to "${text}": ${'streamed word '.repeat(40)}done.`.split(' ')
    words.forEach((word, i) => {
      setTimeout(() => {
        const event = { type: 'messageChunk', role: 'agent', text: `${i ? ' ' : ''}${word}` }
        persist('message', event)
        send(event)
      }, 50 + i * 10)
    })
    setTimeout(() => {
      const end = { type: 'turnEnd', stop_reason: 'end_turn', error: null }
      persist('turn_end', end)
      send(end)
      const lead = fixtureSnapshot.persisted.threads.find((row) => row.id === threadId)
      if (lead) lead.status = 'idle'
      emitStateChanged()
    }, 80 + words.length * 10)
  }

  const invoke = async (cmd: string, args: InvokeArgs = {}) => {
    if (cmd === 'get_snapshot') return fixtureSnapshot
    if (cmd === 'detect_engines') return fixtureEngines
    if (cmd === 'list_repo_branches') {
      return { branches: ['main', 'develop', 'feat/oauth-login'] }
    }
    if (cmd === 'subscribe_diffs' && args.workspaceId === fixtureDiff.workspaceId) {
      const channel = args.channel as { id: number }
      queueMicrotask(() => callbacks.get(channel.id)?.({ index: 0, message: { workspaceId: fixtureDiff.workspaceId, path: '', diff: fixtureDiff } }))
      return nextEventId++
    }
    if (cmd === 'subscribe_agent_events') {
      const channel = args.channel as { id: number }
      agentChannels.set(args.threadId as string, { id: channel.id, index: 0 })
      return nextEventId++
    }
    if (cmd === 'send_thread_prompt' && args.threadId === 'th-lead') {
      const lead = fixtureSnapshot.persisted.threads.find((row) => row.id === 'th-lead')
      if (lead) lead.status = 'running'
      streamReply('th-lead', String(args.text))
      return null
    }
    if (cmd === 'join_workspace_thread') {
      const input = args.input as { workspaceId: string; title: string; engine: string }
      const threadId = `th-${Date.now()}`
      const lead = fixtureSnapshot.persisted.threads[0]!
      fixtureSnapshot.persisted.threads.push({ ...lead, id: threadId, workspaceId: input.workspaceId, title: input.title, status: 'idle' })
      emitStateChanged()
      return { threadId }
    }
    if (cmd === 'close_workspace_thread') {
      const persisted = fixtureSnapshot.persisted
      persisted.threads = persisted.threads.filter((row) => row.id !== args.threadId)
      emitStateChanged()
      return null
    }
    if (cmd === 'get_metrics') return fixtureSnapshot.memory
    if (cmd === 'set_setting') return null
    if (cmd === 'create_scratch')
      return createScratch(args.input as ScratchInput)
    if (cmd === 'end_scratch') {
      const persisted = fixtureSnapshot.persisted
      persisted.scratches = persisted.scratches.filter(
        (row) => row.id !== args.scratchId,
      )
      return null
    }
    if (cmd === 'plugin:event|listen') {
      const name = String(args.event)
      eventListeners.set(name, [...(eventListeners.get(name) ?? []), Number(args.handler)])
      return nextEventId++
    }
    if (cmd === 'plugin:event|unlisten') return null
    if (cmd === 'plugin:window|is_focused') return true
    if (cmd === 'plugin:window|set_title') {
      const title = args.value
      if (typeof title === 'string') document.title = title
      return null
    }
    if (cmd.startsWith('plugin:window|')) return null
    if (cmd.startsWith('plugin:')) return null
    return null
  }

  Object.assign(window, {
    __TAURI_INTERNALS__: {
      invoke,
      transformCallback(callback?: (...args: unknown[]) => void, once = false) {
        const id = nextCallbackId++
        callbacks.set(id, (...cbArgs: unknown[]) => {
          if (once) callbacks.delete(id)
          callback?.(...cbArgs)
        })
        return id
      },
      unregisterCallback(id: number) {
        callbacks.delete(id)
      },
      convertFileSrc(filePath: string) {
        return filePath
      },
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main' },
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener() {},
    },
  })

  document.documentElement.dataset.cormuxHarness = 'browser'
}
