import { NO_USAGE } from '$lib/usage/usage'
import type { Thread } from './threads.svelte'
import { IDLE_RUNTIME, type Workspace } from './workspace-model'

export function workspaceFixture(patch: Partial<Workspace> = {}): Workspace {
  return {
    id: 'ws-1',
    repoId: 'repo-1',
    repoPath: '/tmp/repo',
    worktreePath: '/tmp/wt',
    name: 'Auth',
    branch: 'feat/auth',
    base: 'main',
    kind: 'lead',
    lifecycle: 'idle',
    paused: false,
    activityText: 'Idle',
    pendingApprovals: 0,
    createdAtMs: null,
    summary: null,
    prNumber: null,
    prHtmlUrl: null,
    setupFailure: null,
    run: null,
    reviewStatus: null,
    reviewSubmitted: false,
    runtime: IDLE_RUNTIME,
    ...patch,
  }
}

export function threadFixture(patch: Partial<Thread> = {}): Thread {
  return {
    id: 'th-1',
    workspaceId: 'ws-1',
    role: 'Lead',
    ownRole: 'lead',
    engine: 'claude',
    status: 'idle',
    activity: 'Idle',
    pendingApprovals: 0,
    usage: NO_USAGE,
    retired: false,
    parentThreadId: null,
    ...patch,
  }
}
