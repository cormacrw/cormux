import type { FindingStatus } from '$lib/ipc/bindings'
import { appendEvent, shownMessage, storeNow } from '../db'
import { leadThreadFixture, WORKSPACE_ID, workspaceFixture } from '../fixtures'
import { CHANGE, type HarnessRuntime } from '../runtime'
import type { HandlerSlice, Input, Unsupported } from '../types'

export const workspaceUnsupported = [
  'rename_workspace',
  'retry_workspace_provisioning',
  'skip_workspace_provisioning_setup',
  'send_workspace_findings',
  'submit_workspace_review',
  'start_branch_review',
] as const satisfies Unsupported

export function workspaceHandlers(runtime: HarnessRuntime) {
  const { db, scenario, controls, emitStateChanged } = runtime
  let created = 0

  /** Like `create_workspace`, minus provisioning: the thread opens with the prompt in it. */
  const createWorkspace = (input: Input<'create_workspace'>['input']) => {
    const n = ++created
    const workspaceId = `ws-harness-${n}`
    const threadId = `th-harness-${n}`
    const createdAt = storeNow()
    const { row, record } = workspaceFixture({
      id: workspaceId,
      repoId: input.repoId,
      name: input.goal.slice(0, 48),
      // The core drafts a branch from the goal when none is given; any `feat/` name stands in.
      branch: input.branch || `feat/harness-${n}`,
      base: input.base ?? 'main',
      worktreePath: `/tmp/cormux-fixture/harness-${n}`,
      createdAtMs: Date.now(),
    })
    db.snapshot.persisted.workspaces.push(row)
    db.snapshot.workspaces.push(record)
    db.snapshot.persisted.threads.push({
      ...leadThreadFixture(threadId, workspaceId),
      sessionId: `session-${threadId}`,
    })
    db.snapshot.workspaceOwnThreads.push({ workspaceId, threadId })
    // A new workspace runs what the fixture workspace runs, and is refused Run as it is.
    const like = <T extends { workspaceId: string }>(rows: T[]) =>
      rows
        .filter((row) => row.workspaceId === WORKSPACE_ID)
        .map((row) => ({ ...row, workspaceId }))
    db.snapshot.workspaceRunCommands.push(
      ...like(db.snapshot.workspaceRunCommands),
    )
    db.snapshot.workspaceRunBlocks.push(...like(db.snapshot.workspaceRunBlocks))
    appendEvent(
      db,
      threadId,
      {
        type: 'messageChunk',
        role: 'user',
        text: shownMessage(
          input.goal,
          (input.attachments ?? []).map((attachment) => attachment.name),
        ),
      },
      createdAt,
    )
    emitStateChanged(CHANGE.state)
    return { workspaceId, threadId }
  }

  return {
    set_workspace_run_command: ({
      workspaceId,
      command,
    }: {
      workspaceId: string
      command: string
    }) => {
      const typed = command.trim()
      const edit = scenario.runCommandEdits?.[typed]
      if (!edit) {
        throw {
          kind: 'Invalid',
          message: `the scenario gives no runCommandEdits for "${typed}"`,
        }
      }
      const { row } = runtime.findWorkspace(workspaceId)
      if (row) row.runCommand = edit.run?.source === 'workspace' ? typed : null
      const snapshot = db.snapshot
      snapshot.workspaceRunCommands = [
        ...snapshot.workspaceRunCommands.filter(
          (run) => run.workspaceId !== workspaceId,
        ),
        ...(edit.run ? [{ workspaceId, ...edit.run }] : []),
      ]
      snapshot.workspaceRunBlocks = [
        ...snapshot.workspaceRunBlocks.filter(
          (block) => block.workspaceId !== workspaceId,
        ),
        ...(edit.runBlock ? [{ workspaceId, reason: edit.runBlock }] : []),
      ]
      emitStateChanged(CHANGE.state)
      return null
    },
    create_workspace: ({ input }: Input<'create_workspace'>) => {
      const created = createWorkspace(input)
      // The first prompt, as `send_thread_prompt` records a later one.
      controls.prompts.push({
        threadId: created.threadId,
        text: input.goal,
        attachments: input.attachments ?? [],
      })
      runtime.threadModels.set(created.threadId, input.model)
      runtime.threadEfforts.set(created.threadId, input.effort)
      return created
    },
    // The fixture workspace stands in for the review workspace the core opens.
    create_review_workspace: () => ({
      workspaceId: WORKSPACE_ID,
      created: true,
    }),
    get_teardown_preview: ({ workspaceId }: { workspaceId: string }) => ({
      workspaceId,
      workspaceName: 'OAuth login',
      branch: 'feat/oauth-login',
      worktreePath: '/tmp/cormux-fixture/oauth',
      appRunning: false,
      deleteBranchDefault: true,
      dataLoss:
        workspaceId === scenario.cleanTeardown
          ? { askFirst: false, warning: null }
          : {
              askFirst: true,
              warning:
                "2 uncommitted file changes will be discarded. This can't be undone.",
            },
    }),
    // As the core's first step; the specs look no further than the row going.
    teardown_workspace: ({ input }: Input<'teardown_workspace'>) => {
      const { record } = runtime.findWorkspace(input.workspaceId)
      if (record) record.status = 'tearingDown'
      emitStateChanged(CHANGE.state)
      return null
    },
    open_workspace_terminal: ({ workspaceId }: { workspaceId: string }) => {
      controls.openedTerminal = workspaceId
      return null
    },
    // Run and Stop move the app's status as the core does; no app is really started.
    control_workspace_app: ({ input }: Input<'control_workspace_app'>) => {
      if (input.action === 'clear') return null
      const apps = db.snapshot.workspaceApps.filter(
        (row) => row.workspaceId !== input.workspaceId,
      )
      const up = input.action !== 'stop'
      apps.push({
        workspaceId: input.workspaceId,
        status: up ? 'starting' : 'stopped',
        port: up ? 5173 : null,
        https: false,
        exitCode: null,
      })
      db.snapshot.workspaceApps = apps
      emitStateChanged(CHANGE.state)
      return null
    },
    // The Output panel subscribes when it opens; only `appRunning` prints to it.
    subscribe_pty: ({
      workspaceId,
      channel,
    }: {
      workspaceId: string
      channel: { id: number }
    }) => {
      runtime.ptyChannels.set(workspaceId, { id: channel.id, index: 0 })
      return runtime.newId()
    },
    dismiss_workspace_findings: ({
      input,
    }: Input<'dismiss_workspace_findings'>) => {
      const [from, to]: [FindingStatus, FindingStatus] = input.dismissed
        ? ['open', 'dismissed']
        : ['dismissed', 'open']
      for (const row of db.snapshot.persisted.findings) {
        if (
          row.workspaceId === input.workspaceId &&
          input.findingIds.includes(row.id) &&
          row.status === from
        ) {
          row.status = to
        }
      }
      emitStateChanged(CHANGE.state)
      return null
    },
  } satisfies HandlerSlice
}
