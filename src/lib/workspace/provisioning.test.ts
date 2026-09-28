import { describe, expect, it } from 'vitest'
import {
  isWorkspaceProvisioning,
  threadActivityForProvisioning,
  workspaceActivityFromRecord,
} from './provisioning'

describe('provisioning copy', () => {
  it('maps thread live row subtitles', () => {
    expect(
      threadActivityForProvisioning(
        'provisioning',
        'Running worktree setup…',
        1,
      ),
    ).toBe('Setting up the worktree')
    expect(
      threadActivityForProvisioning('provisioning', 'Starting agent…', 2),
    ).toBe('Starting agent…')
  })

  it('detects provisioning lifecycles', () => {
    expect(isWorkspaceProvisioning('provisioning')).toBe(true)
    expect(isWorkspaceProvisioning('running')).toBe(false)
  })

  it('prefers record activity when present', () => {
    expect(
      workspaceActivityFromRecord(
        'provisioning',
        {
          id: 'ws',
          repoId: 'r',
          repoPath: '',
          name: 'n',
          branch: 'b',
          base: 'main',
          worktreePath: '/tmp',
          status: 'provisioning',
          version: 1,
          activity: 'Joining worktree…',
          provStep: 1,
          setupFailedCommand: null,
          setupFailedExitCode: null,
        },
        null,
      ),
    ).toBe('Joining worktree…')
  })
})
