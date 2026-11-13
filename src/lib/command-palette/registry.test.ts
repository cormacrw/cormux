import { describe, expect, it } from 'vitest'
import { threadFixture, workspaceFixture } from '$lib/state/fixtures'
import { IDLE_RUNTIME } from '$lib/state/workspace-model'
import { appControlsFor } from '$lib/run-app/controls'
import type { Thread } from '$lib/state/threads.svelte'
import { buildPaletteCommands } from './registry'

function threadsOf(rows: Thread[]) {
  return (workspaceId: string) =>
    rows.filter((thread) => thread.workspaceId === workspaceId)
}

describe('buildPaletteCommands', () => {
  it('includes core action and workspace commands', () => {
    const labels = buildPaletteCommands({
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
      threadsOf: threadsOf([threadFixture()]),
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
      prNumberOf: () => null,
      referencesOf: () => [],
      appControlsFor: () =>
        appControlsFor({
          appStatus: 'stopped',
          runBlock: null,
          runCommand: 'pnpm dev',
        }),
      findingsAvailable: () => true,
      needsYou: () => false,
    }).map((c) => c.label)

    expect(labels).toContain('New thread')
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
    const labels = buildPaletteCommands({
      reduceMotion: false,
      clickupConfigured: false,
      workspaces: [workspaceFixture()],
      scratches: [],
      bangs: [],
      threadsOf: threadsOf([
        threadFixture({ id: 'th-old', role: 'Reviewer', retired: true }),
        threadFixture(),
      ]),
      repos: [],
      prNumberOf: () => null,
      referencesOf: () => [],
      appControlsFor: () =>
        appControlsFor({
          appStatus: 'stopped',
          runBlock: 'No run command for this workspace.',
          runCommand: '',
        }),
      findingsAvailable: () => false,
      needsYou: () => false,
    }).map((c) => c.label)

    expect(labels).toContain('Open Reviewer in Auth')
    expect(labels).not.toContain('Open Lead in Auth')
  })

  it('offers Findings only where the Findings tab is on offer', () => {
    const labels = buildPaletteCommands({
      reduceMotion: false,
      clickupConfigured: false,
      workspaces: [
        workspaceFixture(),
        workspaceFixture({ id: 'ws-2', name: 'Billing' }),
      ],
      scratches: [],
      bangs: [],
      threadsOf: () => [],
      repos: [],
      prNumberOf: () => null,
      referencesOf: () => [],
      appControlsFor: () =>
        appControlsFor({
          appStatus: 'stopped',
          runBlock: 'No run command for this workspace.',
          runCommand: '',
        }),
      findingsAvailable: (workspace) => workspace.id === 'ws-2',
      needsYou: () => false,
    }).map((c) => c.label)

    expect(labels).toContain('Open Findings in Billing')
    expect(labels).not.toContain('Open Findings in Auth')
  })

  it("puts a workspace's PR in its meta, and offers no PR review commands", () => {
    const commands = buildPaletteCommands({
      reduceMotion: false,
      clickupConfigured: false,
      workspaces: [
        workspaceFixture(),
        workspaceFixture({ id: 'ws-2', name: 'Billing' }),
      ],
      scratches: [],
      bangs: [],
      threadsOf: () => [],
      repos: [],
      prNumberOf: (workspace) => (workspace.id === 'ws-2' ? 482 : null),
      referencesOf: () => [],
      appControlsFor: () =>
        appControlsFor({
          appStatus: 'stopped',
          runBlock: 'No run command for this workspace.',
          runCommand: '',
        }),
      findingsAvailable: () => false,
      needsYou: () => true,
    })

    const billing = commands.find((c) => c.label === 'Open Billing')
    expect(billing?.meta).toBe('PR #482 · Needs Attention')
    expect(commands.find((c) => c.label === 'Open Auth')?.meta).toBe(
      'Needs Attention',
    )
    expect(commands.some((c) => c.label.startsWith('Review PR'))).toBe(false)
  })

  it("lets a thread's references find it: the workspace's live thread, and a retired one", () => {
    const ticket = {
      id: 1,
      threadId: 'th-1',
      name: 'CU-86b2 Session timeout',
      url: 'https://app.clickup.com/t/86b2abc',
      createdAt: '2026-11-01 10:00:00',
    }
    const doc = { ...ticket, id: 2, threadId: 'th-old', name: 'Auth spec' }
    const commands = buildPaletteCommands({
      reduceMotion: false,
      clickupConfigured: false,
      workspaces: [workspaceFixture()],
      scratches: [],
      bangs: [],
      threadsOf: threadsOf([
        threadFixture({ id: 'th-old', role: 'Reviewer', retired: true }),
        threadFixture({ id: 'th-1' }),
      ]),
      repos: [],
      prNumberOf: () => null,
      referencesOf: (threadId) =>
        [ticket, doc].filter((row) => row.threadId === threadId),
      appControlsFor: () =>
        appControlsFor({
          appStatus: 'stopped',
          runBlock: 'No run command for this workspace.',
          runCommand: '',
        }),
      findingsAvailable: () => false,
      needsYou: () => false,
    })

    expect(
      commands
        .find((c) => c.label === 'Open Auth')
        ?.references?.map((r) => r.name),
    ).toEqual(['CU-86b2 Session timeout'])
    expect(
      commands
        .find((c) => c.label === 'Open Reviewer in Auth')
        ?.references?.map((r) => r.name),
    ).toEqual(['Auth spec'])
  })
})
