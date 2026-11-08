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
      base:
        input.base ||
        db.snapshot.persisted.repos.find((repo) => repo.id === input.repoId)
          ?.defaultBranch ||
        'main',
      worktreePath: `/tmp/cormux-fixture/harness-${n}`,
      createdAtMs: Date.now(),
    })
    db.snapshot.persisted.workspaces.push(row)
    db.snapshot.workspaces.push(record)
    db.snapshot.persisted.threads.push(leadThreadFixture(threadId, workspaceId))
    db.snapshot.workspaceOwnThreads.push({ workspaceId, threadId })
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
      const { row } = runtime.findWorkspace(workspaceId)
      // As the core: no override for a blank command or the repo's own one again.
      const repo = db.snapshot.persisted.repos.find(
        (repo) => repo.id === row?.repoId,
      )
      const typed = command.trim()
      if (row) {
        row.runCommand =
          typed && typed !== repo?.runCommand?.trim() ? typed : null
      }
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
    // As the core: the PR's open review workspace, else the fixture workspace as if started.
    create_review_workspace: ({ input }: Input<'create_review_workspace'>) => {
      const existing = db.snapshot.persisted.workspaces.find(
        (row) =>
          row.kind === 'review' &&
          row.repoId === input.repoId &&
          row.prNumber === input.prNumber,
      )
      return existing
        ? { workspaceId: existing.id, created: false }
        : { workspaceId: WORKSPACE_ID, created: true }
    },
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
    // The sidebar already marked the row as deleting, which is all the specs look at.
    teardown_workspace: () => null,
    open_workspace_terminal: ({ workspaceId }: { workspaceId: string }) => {
      controls.openedTerminal = workspaceId
      return null
    },
    // Run and Stop only move the header's own state; no app is really started.
    control_workspace_app: () => null,
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
