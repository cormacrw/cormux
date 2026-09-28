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
    settings: [
      { key: 'defaultEngine', value: 'cursor' },
      { key: 'defaultBase', value: 'main' },
    ],
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
    timeline: [],
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
