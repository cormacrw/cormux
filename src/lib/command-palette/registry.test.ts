import type { PullRequest } from '$lib/state/prs.svelte'
import { describe, expect, it } from 'vitest'
import { NO_USAGE } from '$lib/usage/usage'
import { buildPaletteCommandsFromState } from './build-registry'

const noop = () => {}

describe('buildPaletteCommandsFromState', () => {
  it('includes core action and workspace commands', () => {
    const labels = buildPaletteCommandsFromState(
      {
        reduceMotion: false,
        clickupConfigured: false,
        workspaces: [
          {
            id: 'ws-1',
            name: 'Auth',
            branch: 'feat/auth',
            lifecycle: 'idle',
            paused: false,
            activityText: 'Idle',
            pendingApprovals: 0,
            cardStatus: 'idle',
            createdAtMs: null,
            summary: null,
            summaryAtMs: null,
            summarySource: 'Haiku 4.5',
            kind: null,
            prNumber: null,
            prHtmlUrl: null,
            modifiedFiles: 0,
            provStep: 0,
            setupFailedCommand: null,
            setupFailedExitCode: null,
          },
        ],
        scratches: [
          {
            id: 'scratch-1',
            title: 'Why the webhook signature fails',
            repoId: 'repo-1',
            threadId: 'th-s',
            engine: 'claude',
            createdAtMs: null,
          },
        ],
        bangs: [
          { id: 'bang-1', name: 'triage', prompt: 'Check CI' },
          { id: 'bang-2', name: 'empty', prompt: '' },
        ],
        threads: [
          {
            id: 'th-1',
            workspaceId: 'ws-1',
            role: 'Lead',
            engine: 'claude',
            status: 'idle',
            paused: false,
            activity: 'Idle',
            pendingApprovals: 0,
            retired: false,
            usage: NO_USAGE,
          },
        ],
        repos: [
          {
            id: 'repo-1',
            path: '/tmp',
            name: 'app',
            defaultBranch: 'main',
            setupCommands: '',
            runCommand: 'pnpm dev',
            singleInstance: false,
          },
        ],
        pullRequests: [
          { id: 'acme/app#482', num: 482, title: 'Fix login' } as PullRequest,
        ],
        records: [
          {
            id: 'ws-1',
            repoId: 'repo-1',
            repoPath: '/tmp',
            name: 'Auth',
            branch: 'feat',
            base: 'main',
            worktreePath: '/tmp/wt',
            status: 'idle',
            version: 1,
            activity: 'Idle',
            provStep: 0,
            setupFailedCommand: null,
            setupFailedExitCode: null,
          },
        ],
        runtimeFor: () => ({
          appStatus: 'stopped',
          port: null,
          behind: 2,
        }),
        repoFor: (id) =>
          id === 'repo-1'
            ? {
                id: 'repo-1',
                path: '/tmp',
                name: 'app',
                defaultBranch: 'main',
                setupCommands: '',
                runCommand: 'pnpm dev',
                singleInstance: false,
              }
            : undefined,
        recordFor: (id) =>
          id === 'ws-1'
            ? {
                id: 'ws-1',
                repoId: 'repo-1',
                repoPath: '/tmp',
                name: 'Auth',
                branch: 'feat',
                base: 'main',
                worktreePath: '/tmp/wt',
                status: 'idle',
                version: 1,
                activity: 'Idle',
                provStep: 0,
                setupFailedCommand: null,
                setupFailedExitCode: null,
              }
            : undefined,
      },
      {
        openNewThread: noop,
        openScratch: noop,
        runBang: noop,
        openRepo: noop,
        reviewPullRequest: noop,
        openSprint: noop,
        openSettings: noop,
        openWorkspace: noop,
        toggleReduceMotion: noop,
        openSettingsSection: noop,
        openWorkspaceThread: noop,
        openWorkspaceFindings: noop,
        runWorkspaceApp: noop,
        pullWorkspace: noop,
      },
    ).map((c) => c.label)

    expect(labels).toContain('New thread')
    expect(labels).toContain('Review PR #482 Fix login')
    // No ClickUp key, no sprint page.
    expect(labels).not.toContain('Go to sprint')
    expect(labels).toContain('!triage')
    expect(labels).not.toContain('!empty')
    expect(labels).not.toContain('Empty')
    expect(labels).toContain('Open Why the webhook signature fails')
    expect(labels).toContain('Open Auth')
    expect(labels).toContain('Run app in Auth')
    expect(labels).toContain('Pull 2 commits from main into Auth')
    expect(labels).toContain('Open Findings in Auth')
  })
})
