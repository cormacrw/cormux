import type { PullRequest } from '$lib/state/prs.svelte'
import { describe, expect, it } from 'vitest'
import { threadFixture, workspaceFixture } from '$lib/state/fixtures'
import { IDLE_RUNTIME } from '$lib/state/workspace-model'
import { appControlsFor } from '$lib/run-app/controls'
import { buildPaletteCommandsFromState } from './build-registry'

const noop = () => {}
const actions = {
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
  controlApp: noop,
  pullWorkspace: noop,
}

describe('buildPaletteCommandsFromState', () => {
  it('includes core action and workspace commands', () => {
    const labels = buildPaletteCommandsFromState(
      {
        reduceMotion: false,
        clickupConfigured: false,
        workspaces: [
          workspaceFixture({ runtime: { ...IDLE_RUNTIME, behind: 2 } }),
        ],
        scratches: [
          {
            id: 'scratch-1',
            title: 'Why the webhook signature fails',
            repoId: 'repo-1',
            threadId: 'th-s',
            engine: 'claude',
            status: 'idle',
            createdAtMs: null,
          },
        ],
        bangs: [
          { id: 'bang-1', name: 'triage', prompt: 'Check CI' },
          { id: 'bang-2', name: 'empty', prompt: '' },
        ],
        threads: [threadFixture()],
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
        appControlsFor: () =>
          appControlsFor({
            appStatus: 'stopped',
            runBlock: null,
            runCommand: 'pnpm dev',
          }),
        findingsAvailable: () => true,
      },
      actions,
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

  it("offers a workspace's retired threads, not its live one", () => {
    const labels = buildPaletteCommandsFromState(
      {
        reduceMotion: false,
        clickupConfigured: false,
        workspaces: [workspaceFixture()],
        scratches: [],
        bangs: [],
        threads: [
          threadFixture({ id: 'th-old', role: 'Reviewer', retired: true }),
          threadFixture(),
        ],
        repos: [],
        pullRequests: [],
        appControlsFor: () =>
          appControlsFor({
            appStatus: 'stopped',
            runBlock: 'No run command for this workspace.',
            runCommand: '',
          }),
        findingsAvailable: () => false,
      },
      actions,
    ).map((c) => c.label)

    expect(labels).toContain('Open Reviewer in Auth')
    expect(labels).not.toContain('Open Lead in Auth')
  })

  it('offers Findings only where the Findings tab is on offer', () => {
    const labels = buildPaletteCommandsFromState(
      {
        reduceMotion: false,
        clickupConfigured: false,
        workspaces: [
          workspaceFixture(),
          workspaceFixture({ id: 'ws-2', name: 'Billing' }),
        ],
        scratches: [],
        bangs: [],
        threads: [],
        repos: [],
        pullRequests: [],
        appControlsFor: () =>
          appControlsFor({
            appStatus: 'stopped',
            runBlock: 'No run command for this workspace.',
            runCommand: '',
          }),
        findingsAvailable: (workspace) => workspace.id === 'ws-2',
      },
      actions,
    ).map((c) => c.label)

    expect(labels).toContain('Open Findings in Billing')
    expect(labels).not.toContain('Open Findings in Auth')
  })
})
