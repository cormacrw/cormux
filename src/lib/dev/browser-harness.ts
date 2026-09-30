import {
  fixtureDiff,
  fixtureEngines,
  fixtureSnapshot,
} from './fixture-snapshot'

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
      // The core drafts a blank title from the prompt; Haiku's rename is not stubbed.
      title:
        input.title.trim() ||
        (prompt ? prompt.charAt(0).toUpperCase() + prompt.slice(1, 64) : '') ||
        'Untitled scratch',
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
    push('message', {
      type: 'messageChunk',
      role: 'user',
      text: `Question ${turn}`,
    })
    push('message', {
      type: 'messageChunk',
      role: 'agent',
      text: `Answer ${turn}`,
    })
    push('turn_end', { type: 'turnEnd', stop_reason: 'end_turn', error: null })
  }
  // Switching away from a branch and back writes two notes with the same title.
  for (const branch of ['feat/test', 'feat/colors', 'feat/test']) {
    push('tool', {
      icon: 'branch',
      title: `Switched to \`${branch}\``,
      detail: 'Checked out in this worktree',
    })
  }
}

export function installBrowserHarness() {
  if (typeof window === 'undefined' || isRealTauri()) return
  const longThread = Number(
    new URLSearchParams(window.location.search).get('longThread'),
  )
  if (longThread > 0) seedLongThread(longThread)
  // Tests can inject a real thread's events (as exported from cormux.db) for the Lead thread.
  const injected = (
    window as {
      __HARNESS_TIMELINE__?: typeof fixtureSnapshot.persisted.timeline
    }
  ).__HARNESS_TIMELINE__
  if (injected) {
    const persisted = fixtureSnapshot.persisted
    persisted.timeline = [
      ...persisted.timeline.filter((row) => row.threadId !== 'th-lead'),
      ...injected,
    ]
  }
  // Tests can inject review findings; this makes the fixture workspace a finished review.
  const injectedFindings = (
    window as {
      __HARNESS_REVIEW_FINDINGS__?: typeof fixtureSnapshot.persisted.findings
    }
  ).__HARNESS_REVIEW_FINDINGS__
  if (injectedFindings) {
    const persisted = fixtureSnapshot.persisted
    const row = persisted.workspaces.find(
      (workspace) => workspace.id === 'ws-auth',
    )!
    row.kind = 'review'
    row.prNumber = 2
    persisted.threads.find((thread) => thread.id === 'th-lead')!.title =
      'Reviewer'
    persisted.findings = injectedFindings
    persisted.settings = [
      ...persisted.settings,
      { key: 'review:ws-auth/status', value: 'ready' },
    ]
  }
  // Tests can inject synced PRs; this also flips GitHub to connected.
  const injectedPrs = (
    window as {
      __HARNESS_PRS__?: typeof fixtureSnapshot.persisted.pullRequests
    }
  ).__HARNESS_PRS__
  if (injectedPrs) {
    fixtureSnapshot.persisted.pullRequests = injectedPrs
    fixtureSnapshot.githubAuthConfigured = true
    fixtureSnapshot.prSyncedAt = String(Math.floor(Date.now() / 1000))
  }

  // Lets tests raise a toast of any tone without driving a flow that produces it.
  void import('$lib/feedback/show-toast').then(({ showToast }) => {
    ;(
      window as { __HARNESS_SHOW_TOAST__?: typeof showToast }
    ).__HARNESS_SHOW_TOAST__ = showToast
  })

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
    let seq = Math.max(
      0,
      ...timeline
        .filter((row) => row.threadId === threadId)
        .map((row) => row.seq),
    )
    const persist = (kind: string, event: object) => {
      seq += 1
      timeline.push({
        id: 900_000 + seq,
        threadId,
        seq,
        kind,
        payload: JSON.stringify(event),
        createdAt: '2026-09-28 12:00:00',
      })
    }
    const send = (event: object) => {
      const channel = agentChannels.get(threadId)
      if (!channel) return
      callbacks.get(channel.id)?.({ index: channel.index++, message: event })
    }
    persist('message', { type: 'messageChunk', role: 'user', text })
    // One tool call first, the way agents read before they answer.
    setTimeout(() => {
      const read = {
        type: 'toolCall',
        id: `harness-read-${seq}`,
        title: 'Read',
        name: null,
        kind: 'read',
        status: 'completed',
        locations: ['src/lib/auth.ts'],
        detail: 'src/lib/auth.ts',
      }
      persist('tool', read)
      send(read)
    }, 40)
    // `?slowReply=1` streams a long reply at a real agent's pace instead of a quick one.
    const slow = new URLSearchParams(window.location.search).has('slowReply')
    const wordMs = slow ? 40 : 10
    const body = slow
      ? Array.from(
          { length: 12 },
          (_, n) => `Paragraph ${n + 1}: ${'streamed word '.repeat(25)}`,
        ).join('\n\n')
      : 'streamed word '.repeat(40)
    const words = `Reply to "${text}": ${body}done.`.split(' ')
    words.forEach((word, i) => {
      setTimeout(
        () => {
          const event = {
            type: 'messageChunk',
            role: 'agent',
            text: `${i ? ' ' : ''}${word}`,
          }
          persist('message', event)
          send(event)
        },
        50 + i * wordMs,
      )
    })
    setTimeout(
      () => {
        const end = { type: 'turnEnd', stop_reason: 'end_turn', error: null }
        persist('turn_end', end)
        send(end)
        const lead = fixtureSnapshot.persisted.threads.find(
          (row) => row.id === threadId,
        )
        if (lead) lead.status = 'idle'
        emitStateChanged()
      },
      80 + words.length * wordMs,
    )
  }

  let diffChannel: { id: number; index: number } | null = null
  const sendDiff = (base: string | null) => {
    if (!diffChannel) return
    const diff = { ...fixtureDiff, base }
    callbacks.get(diffChannel.id)?.({
      index: diffChannel.index++,
      message: { workspaceId: fixtureDiff.workspaceId, path: '', diff },
    })
  }

  // gh-stack stand-in: `?stack=none` starts unstacked, `?stack=unavailable` has no extension.
  const stackMode = new URLSearchParams(window.location.search).get('stack')
  const stackBranches: string[] =
    stackMode === 'none' || stackMode === 'unavailable'
      ? []
      : ['feat/oauth-api', 'feat/oauth-login', 'feat/oauth-ui']
  const stackStats: Record<string, [number, number, number, number]> = {
    'feat/oauth-api': [212, 38, 4, 6],
    'feat/oauth-login': [96, 12, 2, 3],
    'feat/oauth-ui': [341, 57, 5, 9],
  }
  // PRs are opened by hand; the bottom branch already has one.
  const stackPrs: Record<string, number> = { 'feat/oauth-api': 101 }
  const setFixtureBranch = (workspaceId: string, branch: string) => {
    const row = fixtureSnapshot.persisted.workspaces.find(
      (workspace) => workspace.id === workspaceId,
    )
    const record = fixtureSnapshot.workspaces.find(
      (workspace) => workspace.id === workspaceId,
    )
    if (row) row.branch = branch
    if (record) record.branch = branch
    emitStateChanged()
  }
  const currentBranch = (workspaceId: string) =>
    fixtureSnapshot.workspaces.find((workspace) => workspace.id === workspaceId)
      ?.branch ?? 'main'
  const stackFor = (workspaceId: string) => {
    const current = currentBranch(workspaceId)
    const base = {
      workspaceId,
      trunk: 'main',
      currentBranch: current,
      message: null,
      branches: [],
    }
    if (stackMode === 'unavailable') {
      return {
        ...base,
        status: 'unavailable',
        message:
          "The gh-stack extension isn't installed. Run: gh extension install github/gh-stack",
      }
    }
    if (!stackBranches.includes(current))
      return { ...base, status: 'notStacked' }
    const branches = stackBranches.map((name, index) => {
      const [additions, deletions, commits, files] = stackStats[name] ?? [
        0, 0, 0, 0,
      ]
      const number = stackPrs[name]
      return {
        name,
        parent: stackBranches[index - 1] ?? 'main',
        files,
        additions,
        deletions,
        commits,
        current: name === current,
        merged: false,
        queued: false,
        needsRebase: name === 'feat/oauth-ui',
        pr: number
          ? {
              number,
              url: `https://github.com/acme/my-app/pull/${number}`,
              state: 'OPEN',
            }
          : null,
      }
    })
    return { ...base, status: 'stacked', branches }
  }

  const invoke = async (cmd: string, args: InvokeArgs = {}) => {
    if (cmd === 'get_snapshot') return fixtureSnapshot
    if (cmd === 'get_workspace_stack') return stackFor(String(args.workspaceId))
    if (cmd === 'switch_workspace_branch') {
      const input = args.input as { workspaceId: string; branch: string }
      setFixtureBranch(input.workspaceId, input.branch)
      return null
    }
    if (cmd === 'add_stack_branch') {
      const input = args.input as { workspaceId: string; branch: string }
      const current = currentBranch(input.workspaceId)
      if (!stackBranches.includes(current) && current !== 'main')
        stackBranches.push(current)
      stackBranches.push(input.branch)
      setFixtureBranch(input.workspaceId, input.branch)
      return null
    }
    if (cmd === 'push_stack') {
      await new Promise((resolve) => setTimeout(resolve, 300))
      return null
    }
    if (cmd === 'sync_stack') return null
    if (cmd === 'draft_pr_why') {
      return {
        workspaceId: args.workspaceId,
        text: 'Lets people sign in with their Google account.',
        fromLlm: false,
      }
    }
    if (cmd === 'create_todo') {
      const todo = {
        id: `todo-harness-${nextEventId++}`,
        title: String(args.title).trim(),
        pinned: false,
      }
      fixtureSnapshot.persisted.todos.push(todo)
      return todo
    }
    if (cmd === 'delete_todo') {
      const persisted = fixtureSnapshot.persisted
      persisted.todos = persisted.todos.filter((row) => row.id !== args.todoId)
      return null
    }
    if (cmd === 'set_todo_pinned') {
      const todo = fixtureSnapshot.persisted.todos.find(
        (row) => row.id === args.todoId,
      )
      if (todo) todo.pinned = Boolean(args.pinned)
      return null
    }
    if (cmd === 'detect_engines') return fixtureEngines
    if (cmd === 'list_repo_branches') {
      return { branches: ['main', 'develop', 'feat/oauth-login'] }
    }
    if (
      cmd === 'subscribe_diffs' &&
      args.workspaceId === fixtureDiff.workspaceId
    ) {
      const channel = args.channel as { id: number }
      diffChannel = { id: channel.id, index: 0 }
      queueMicrotask(() => sendDiff(null))
      return nextEventId++
    }
    // `?slowDiff=1` holds diff fetches long enough to see the Changes splash.
    if (cmd === 'refresh_workspace_diff' || cmd === 'set_workspace_diff_base') {
      if (new URLSearchParams(window.location.search).has('slowDiff')) {
        await new Promise((resolve) => setTimeout(resolve, 1500))
      }
      if (
        cmd === 'set_workspace_diff_base' &&
        args.workspaceId === fixtureDiff.workspaceId
      ) {
        setTimeout(() => sendDiff((args.base as string | null) ?? null), 100)
      }
      return null
    }
    if (cmd === 'subscribe_agent_events') {
      const channel = args.channel as { id: number }
      agentChannels.set(args.threadId as string, { id: channel.id, index: 0 })
      return nextEventId++
    }
    if (cmd === 'send_thread_prompt' && args.threadId === 'th-lead') {
      const lead = fixtureSnapshot.persisted.threads.find(
        (row) => row.id === 'th-lead',
      )
      if (lead) lead.status = 'running'
      streamReply('th-lead', String(args.text))
      return null
    }
    if (cmd === 'new_thread_session') {
      const threadId = String(args.threadId)
      const timeline = fixtureSnapshot.persisted.timeline
      const seq =
        Math.max(
          0,
          ...timeline
            .filter((row) => row.threadId === threadId)
            .map((row) => row.seq),
        ) + 1
      const event = {
        type: 'toolCall',
        id: `control-${seq}`,
        title: 'Started a new session',
        name: null,
        kind: 'other',
        status: 'completed',
        locations: [],
        detail: null,
      }
      timeline.push({
        id: 800_000 + seq,
        threadId,
        seq,
        kind: 'tool',
        payload: JSON.stringify(event),
        createdAt: '2026-09-28 12:00:00',
      })
      const thread = fixtureSnapshot.persisted.threads.find(
        (row) => row.id === threadId,
      )
      if (thread) thread.status = 'idle'
      emitStateChanged()
      return null
    }
    if (cmd === 'join_workspace_thread') {
      const input = args.input as {
        workspaceId: string
        title: string
        engine: string
      }
      const threadId = `th-${Date.now()}`
      const lead = fixtureSnapshot.persisted.threads[0]!
      fixtureSnapshot.persisted.threads.push({
        ...lead,
        id: threadId,
        workspaceId: input.workspaceId,
        title: input.title,
        status: 'idle',
      })
      emitStateChanged()
      return { threadId }
    }
    if (cmd === 'close_workspace_thread') {
      const persisted = fixtureSnapshot.persisted
      persisted.threads = persisted.threads.filter(
        (row) => row.id !== args.threadId,
      )
      emitStateChanged()
      return null
    }
    if (cmd === 'get_metrics') return fixtureSnapshot.memory
    if (cmd === 'set_setting') {
      const { key, value } = args.input as { key: string; value: string }
      const persisted = fixtureSnapshot.persisted
      persisted.settings = [
        ...persisted.settings.filter((row) => row.key !== key),
        { key, value },
      ]
      return null
    }
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
      eventListeners.set(name, [
        ...(eventListeners.get(name) ?? []),
        Number(args.handler),
      ])
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
    if (cmd === 'create_review_workspace') {
      // Slow enough for tests to see the button's loading state; opens the fixture workspace.
      await new Promise((resolve) => setTimeout(resolve, 400))
      return { workspaceId: 'ws-auth', created: true }
    }
    if (cmd === 'plugin:opener|open_url') {
      const opened = ((
        window as { __HARNESS_OPENED__?: string[] }
      ).__HARNESS_OPENED__ ??= [])
      opened.push(String(args.url))
      return null
    }
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
