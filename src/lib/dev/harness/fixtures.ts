import type {
  EngineStatus,
  Snapshot,
  ThreadRow,
  WorkspaceRecord,
  WorkspaceRow,
} from '$lib/ipc/bindings'
import { defaultSettings } from '$lib/settings/defaults'
import { clickupBoard } from './clickup-fixtures'
import type { StoredEventRow } from './db'

export const REPO_ID = 'my-app'
const REPO_PATH = '/tmp/cormux-fixture/my-app'
/** The one workspace the app starts with, "OAuth login". */
export const WORKSPACE_ID = 'ws-auth'
/** That workspace's own thread: its Lead, or its Reviewer once a scenario makes it a review. */
export const LEAD_THREAD_ID = 'th-lead'
const SCRATCH_THREAD_ID = 'th-scratch-webhook'

export const fixtureEngines: EngineStatus[] = [
  {
    kind: 'claude',
    installed: true,
    binary: '/usr/local/bin/claude',
    version: '1.0.0',
    signedIn: true,
  },
]

type NewWorkspace = {
  id: string
  repoId: string
  name: string
  branch: string
  base: string
  worktreePath: string
  createdAtMs: number
}

/** A Lead workspace as the core stores it and as its snapshot record shows it, idle. */
export function workspaceFixture(fields: NewWorkspace): {
  row: WorkspaceRow
  record: WorkspaceRecord
} {
  const { createdAtMs, ...shared } = fields
  return {
    row: {
      ...shared,
      status: 'idle',
      createdAtMs,
      summary: null,
      summaryAt: null,
      kind: 'lead',
      prNumber: null,
      prHtmlUrl: null,
      archivedAt: null,
      runCommand: null,
    },
    record: {
      ...shared,
      repoPath: REPO_PATH,
      status: 'idle',
      kind: 'lead',
      activity: { kind: 'none' },
    },
  }
}

/** A workspace's own Lead thread, idle with no session yet. */
export function leadThreadFixture(id: string, workspaceId: string): ThreadRow {
  return {
    id,
    workspaceId,
    title: 'Lead',
    role: 'lead',
    engine: 'claude',
    sessionId: null,
    status: 'idle',
    usedTokens: null,
    contextSize: null,
    costUsd: null,
    retired: false,
    parentThreadId: null,
  }
}

/** Fresh data for one browser session: the core's state, its `thread_events` and the rest. */
export function createFixtures() {
  const oauth = workspaceFixture({
    id: WORKSPACE_ID,
    repoId: REPO_ID,
    name: 'OAuth login',
    branch: 'feat/oauth-login',
    base: 'main',
    worktreePath: '/tmp/cormux-fixture/oauth',
    createdAtMs: 0,
  })
  const snapshot: Snapshot = {
    version: 1,
    // The core's defaults.
    settings: defaultSettings(),
    persisted: {
      reviews: [],
      repos: [
        {
          id: REPO_ID,
          path: REPO_PATH,
          name: 'my-app',
          defaultBranch: 'main',
          setupCommands: '',
          runCommand: null,
          singleInstance: false,
        },
      ],
      workspaces: [oauth.row],
      threads: [leadThreadFixture(LEAD_THREAD_ID, WORKSPACE_ID)],
      scratches: [
        {
          id: 'scratch-webhook',
          repoId: REPO_ID,
          title: 'Why the webhook signature fails',
          threadId: SCRATCH_THREAD_ID,
          engine: 'claude',
          status: 'idle',
          createdAtMs: Date.UTC(2026, 8, 28, 12),
        },
      ],
      threadActivity: [],
      approvals: [],
      findings: [],
      entities: [],
      pullRequests: [],
    },
    workspaces: [oauth.record],
    workspaceGit: [],
    memory: { totalBytes: 1_400_000_000 },
    pendingApprovals: 0,
    githubAuthConfigured: false,
    clickupConfigured: false,
    workspaceApps: [],
    repoRunCommands: [],
    repoCommittedSetups: [],
    workspaceRunCommands: [],
    repoRemovalBlocks: [],
    workspaceBranchLocks: [],
  }
  const timeline: StoredEventRow[] = [
    {
      id: 1,
      threadId: SCRATCH_THREAD_ID,
      seq: 1,
      kind: 'message',
      payload: JSON.stringify({
        type: 'messageChunk',
        role: 'user',
        text: 'Stripe webhooks are failing signature checks on /api/stripe/webhook. Why?',
      }),
      createdAt: '2026-09-28 12:00:00',
    },
    {
      id: 4,
      threadId: SCRATCH_THREAD_ID,
      seq: 2,
      kind: 'message',
      payload: JSON.stringify({
        type: 'messageChunk',
        role: 'thought',
        text: 'Signature failures usually mean the body was parsed before verification. Check how the handler reads the request.',
      }),
      createdAt: '2026-09-28 12:00:02',
    },
    {
      id: 2,
      threadId: SCRATCH_THREAD_ID,
      seq: 3,
      kind: 'tool',
      payload: JSON.stringify({
        type: 'toolCall',
        id: 'read-webhook',
        title: 'Read 2 files',
        role: { kind: 'read' },
        status: 'completed',
        locations: [],
        detail: 'src/routes/api/stripe/webhook/+server.ts, src/lib/stripe.ts',
      }),
      createdAt: '2026-09-28 12:00:05',
    },
    {
      id: 5,
      threadId: SCRATCH_THREAD_ID,
      seq: 4,
      kind: 'tool',
      payload: JSON.stringify({
        type: 'toolCall',
        id: 'search-construct',
        title: 'Searched for constructEvent',
        role: { kind: 'search' },
        status: 'completed',
        locations: [],
        detail: null,
      }),
      createdAt: '2026-09-28 12:00:08',
    },
    {
      id: 6,
      threadId: SCRATCH_THREAD_ID,
      seq: 5,
      kind: 'tool',
      payload: JSON.stringify({
        type: 'toolCall',
        id: 'shell-log',
        title:
          '`git log --oneline -5 -- src/routes/api/stripe/webhook/+server.ts src/lib/stripe.ts`',
        role: {
          kind: 'command',
          command:
            '`git log --oneline -5 -- src/routes/api/stripe/webhook/+server.ts src/lib/stripe.ts`',
          description: null,
        },
        status: 'completed',
        locations: [],
        detail: null,
      }),
      createdAt: '2026-09-28 12:00:12',
    },
    {
      id: 3,
      threadId: SCRATCH_THREAD_ID,
      seq: 6,
      kind: 'message',
      payload: JSON.stringify({
        type: 'messageChunk',
        role: 'agent',
        text: 'The handler calls request.json() before constructEvent. Stripe signs the raw body, so verify against request.text() first. I didn’t edit anything.',
      }),
      createdAt: '2026-09-28 12:00:20',
    },
  ]
  return { snapshot, timeline, diff: oauthDiff(), clickup: clickupBoard() }
}

/** The OAuth workspace's uncommitted changes, as its diff stream sends them. */
function oauthDiff() {
  return {
    workspaceId: WORKSPACE_ID,
    target: null,
    files: [
      {
        path: 'src/auth/session.ts',
        oldPath: null,
        status: 'modified' as const,
        added: 4,
        deleted: 1,
        hunks: [
          {
            header: '@@ -1,5 +1,8 @@ export function getSession',
            body: [
              " import { cookies } from './cookies'",
              '',
              '-export function getSession() {',
              '+export function getSession(provider?: string) {',
              "+  if (provider === 'github') {",
              '+    return cookies.get(`oauth:${provider}`)',
              '+  }',
              "   return cookies.get('session')",
              ' }',
              '',
            ].join('\n'),
          },
        ],
      },
      {
        path: 'src/auth/providers.ts',
        oldPath: null,
        status: 'added' as const,
        added: 60,
        deleted: 0,
        hunks: [
          {
            header: '@@ -0,0 +1,60 @@',
            body:
              Array.from(
                { length: 60 },
                (_, i) => `+export const provider${i + 1} = 'p${i + 1}'`,
              ).join('\n') + '\n',
          },
        ],
      },
    ],
  }
}
