import { shownMessage } from '$lib/composer/attachments'
import { fixtureSnapshot, fixtureTimeline } from '../../fixture-snapshot'
import type { HarnessRuntime } from '../runtime'
import type { HandlerSlice, Input } from '../types'

type WorkspaceInput = Input<'create_workspace'>['input']

let nextWorkspace = 1

/** Mirrors `create_workspace`, minus provisioning: the thread opens with the prompt in it. */
function createWorkspace(input: WorkspaceInput) {
  const n = nextWorkspace++
  const workspaceId = `ws-harness-${n}`
  const threadId = `th-harness-${n}`
  const persisted = fixtureSnapshot.persisted
  const createdAt = new Date().toISOString().slice(0, 19).replace('T', ' ')
  const name = input.goal.slice(0, 48)
  persisted.workspaces.push({
    ...persisted.workspaces[0]!,
    id: workspaceId,
    repoId: input.repoId,
    name,
    branch: input.branch,
    status: 'idle',
    createdAt,
  })
  fixtureSnapshot.workspaces.push({
    ...fixtureSnapshot.workspaces[0]!,
    id: workspaceId,
    repoId: input.repoId,
    name,
    branch: input.branch,
    base: input.base,
  })
  persisted.threads.push({
    ...persisted.threads[0]!,
    id: threadId,
    workspaceId,
    status: 'idle',
  })
  fixtureTimeline.push({
    id: 1000 * n,
    threadId,
    seq: 1,
    kind: 'message',
    payload: JSON.stringify({
      type: 'messageChunk',
      role: 'user',
      text: shownMessage(
        input.goal,
        (input.attachments ?? []).map((attachment) => attachment.name),
      ),
    }),
    createdAt,
  })
  return { workspaceId, threadId }
}

export function workspaceHandlers(runtime: HarnessRuntime) {
  const { scenario, controls, emitStateChanged } = runtime

  return {
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
    create_review_workspace: async () => {
      // Slow enough for tests to see the button's loading state; opens the fixture workspace.
      await new Promise((resolve) => setTimeout(resolve, 400))
      return { workspaceId: 'ws-auth', created: true }
    },
    get_teardown_preview: ({ workspaceId }: { workspaceId: string }) => ({
      workspaceId,
      workspaceName: 'OAuth login',
      engineLabel: 'Claude Code',
      branch: 'feat/oauth-login',
      worktreePath: '/tmp/cormux-fixture/oauth',
      appRunning: false,
      deleteBranchDefault: true,
      dataLoss:
        workspaceId === scenario.cleanTeardown
          ? {
              uncommittedFiles: 0,
              unpushedCommits: 0,
              hasDataLoss: false,
              warning: null,
            }
          : {
              uncommittedFiles: 2,
              unpushedCommits: 0,
              hasDataLoss: true,
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
    // The Output panel subscribes when it opens; a fixture workspace has no process to log.
    subscribe_pty: () => runtime.newId(),
    dismiss_workspace_findings: ({
      input,
    }: Input<'dismiss_workspace_findings'>) => {
      const [from, to] = input.dismissed
        ? ['open', 'dismissed']
        : ['dismissed', 'open']
      for (const row of fixtureSnapshot.persisted.findings) {
        if (
          row.workspaceId === input.workspaceId &&
          input.findingIds.includes(row.id) &&
          row.status === from
        ) {
          row.status = to
        }
      }
      emitStateChanged()
      return null
    },
  } satisfies HandlerSlice
}
