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

export function installBrowserHarness() {
  if (typeof window === 'undefined' || isRealTauri()) return

  const callbacks = new Map<number, (...args: unknown[]) => void>()
  let nextCallbackId = 1
  let nextEventId = 1

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
    if (cmd === 'plugin:event|listen') return nextEventId++
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
