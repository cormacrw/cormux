import type { EngineStatus, Snapshot } from '$lib/ipc/bindings'

export const FIXTURE_REPO_ID = 'my-app'

export const fixtureEngines: EngineStatus[] = [
  {
    kind: 'claude',
    installed: true,
    binary: '/usr/local/bin/claude',
    version: '1.0.0',
    signedIn: true,
  },
  {
    kind: 'cursor',
    installed: true,
    binary: '/usr/local/bin/cursor',
    version: '1.0.0',
    signedIn: true,
  },
  {
    kind: 'codex',
    installed: false,
    binary: null,
    version: null,
    signedIn: null,
  },
  {
    kind: 'gemini',
    installed: false,
    binary: null,
    version: null,
    signedIn: null,
  },
]

export const fixtureSnapshot: Snapshot = {
  version: 1,
  view: 'homebase',
  persisted: {
    settings: [{ key: 'defaultEngine', value: 'cursor' }],
    repos: [
      {
        id: FIXTURE_REPO_ID,
        path: '/tmp/cormux-fixture/my-app',
        name: 'my-app',
        defaultBranch: 'main',
        setupCommands: '',
        runCommand: null,
      },
    ],
    workspaces: [
      {
        id: 'ws-auth',
        repoId: FIXTURE_REPO_ID,
        name: 'OAuth login',
        branch: 'feat/oauth-login',
        worktreePath: '/tmp/cormux-fixture/oauth',
        status: 'idle',
        createdAt: '1700000000',
        summary: null,
        summaryAt: null,
        summarySource: 'Haiku 4.5',
        kind: null,
        prNumber: null,
        prHtmlUrl: null,
        modifiedFiles: 0,
        archivedAt: null,
      },
    ],
    threads: [
      {
        id: 'th-lead',
        workspaceId: 'ws-auth',
        title: 'Lead',
        engine: 'cursor',
        sessionId: null,
        status: 'idle',
        usedTokens: null,
        contextSize: null,
        costUsd: null,
        transcriptReadonly: false,
      },
    ],
    todos: [
      {
        id: 'todo-webhook',
        title: 'Rotate the Stripe webhook secret',
        pinned: true,
      },
      { id: 'todo-changelog', title: 'Draft the 0.4 changelog', pinned: false },
    ],
    scratches: [
      {
        id: 'scratch-webhook',
        repoId: FIXTURE_REPO_ID,
        title: 'Why the webhook signature fails',
        threadId: 'th-scratch-webhook',
        engine: 'claude',
        status: 'idle',
        createdAt: '2026-09-28 12:00:00',
      },
    ],
    timeline: [
      {
        id: 1,
        threadId: 'th-scratch-webhook',
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
        threadId: 'th-scratch-webhook',
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
        threadId: 'th-scratch-webhook',
        seq: 3,
        kind: 'tool',
        payload: JSON.stringify({
          type: 'toolCall',
          id: 'read-webhook',
          title: 'Read 2 files',
          name: 'Read',
          kind: 'read',
          status: 'completed',
          locations: [],
          detail: 'src/routes/api/stripe/webhook/+server.ts, src/lib/stripe.ts',
        }),
        createdAt: '2026-09-28 12:00:05',
      },
      {
        id: 5,
        threadId: 'th-scratch-webhook',
        seq: 4,
        kind: 'tool',
        payload: JSON.stringify({
          type: 'toolCall',
          id: 'search-construct',
          title: 'Searched for constructEvent',
          name: 'Grep',
          kind: 'search',
          status: 'completed',
          locations: [],
          detail: null,
        }),
        createdAt: '2026-09-28 12:00:08',
      },
      {
        id: 6,
        threadId: 'th-scratch-webhook',
        seq: 5,
        kind: 'tool',
        payload: JSON.stringify({
          type: 'toolCall',
          id: 'shell-log',
          title:
            '`git log --oneline -5 -- src/routes/api/stripe/webhook/+server.ts src/lib/stripe.ts`',
          name: null,
          kind: 'execute',
          status: 'completed',
          locations: [],
          detail: null,
        }),
        createdAt: '2026-09-28 12:00:12',
      },
      {
        id: 3,
        threadId: 'th-scratch-webhook',
        seq: 6,
        kind: 'message',
        payload: JSON.stringify({
          type: 'messageChunk',
          role: 'agent',
          text: 'The handler calls request.json() before constructEvent. Stripe signs the raw body, so verify against request.text() first. I didn’t edit anything.',
        }),
        createdAt: '2026-09-28 12:00:20',
      },
    ],
    approvals: [],
    findings: [],
    pullRequests: [],
  },
  workspaces: [
    {
      id: 'ws-auth',
      repoId: FIXTURE_REPO_ID,
      repoPath: '/tmp/cormux-fixture/my-app',
      name: 'OAuth login',
      branch: 'feat/oauth-login',
      base: 'main',
      worktreePath: '/tmp/cormux-fixture/oauth',
      status: 'idle',
      version: 1,
      activity: 'Idle',
      provStep: 0,
      setupFailedCommand: null,
      setupFailedExitCode: null,
    },
  ],
  workspaceGit: [],
  memory: { totalBytes: 1_400_000_000, perWorkspace: [] },
  pendingLiveApprovals: 0,
  githubAuthConfigured: false,
  prSyncedAt: null,
  workspaceApps: [],
}

/** Uncommitted diff the harness streams for the OAuth workspace. */
export const fixtureDiff = {
  workspaceId: 'ws-auth',
  target: null,
  files: [
    {
      path: 'src/auth/session.ts',
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
