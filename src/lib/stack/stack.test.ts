import { describe, expect, it } from 'vitest'
import type { StackBranch, WorkspaceStack } from '$lib/ipc/bindings'
import type { PullRequest } from '$lib/state/prs.svelte'
import {
  canAddToStack,
  commitsLabel,
  pullRequestForBranch,
  stackCardsTopFirst,
  stackSubtitle,
  stackTabAriaLabel,
} from './stack'

function branch(name: string, extra: Partial<StackBranch> = {}): StackBranch {
  return {
    name,
    parent: 'main',
    files: 0,
    additions: 0,
    deletions: 0,
    commits: 0,
    current: false,
    merged: false,
    queued: false,
    needsRebase: false,
    pr: null,
    ...extra,
  }
}

function stack(extra: Partial<WorkspaceStack> = {}): WorkspaceStack {
  return {
    workspaceId: 'ws',
    status: 'stacked',
    message: null,
    trunk: 'main',
    currentBranch: 'b',
    branches: [branch('a', { merged: true }), branch('b', { current: true })],
    ...extra,
  }
}

describe('stack helpers', () => {
  it('lists the top of the stack first', () => {
    expect(stackCardsTopFirst(stack()).map((row) => row.name)).toEqual([
      'b',
      'a',
    ])
    expect(stackCardsTopFirst(undefined)).toEqual([])
  })

  it('only adds from the top of the stack, or to start one', () => {
    expect(canAddToStack(stack())).toBe(true)
    expect(canAddToStack(stack({ currentBranch: 'a' }))).toBe(false)
    expect(canAddToStack(stack({ status: 'notStacked', branches: [] }))).toBe(
      true,
    )
    expect(canAddToStack(stack({ status: 'unavailable', branches: [] }))).toBe(
      false,
    )
    expect(canAddToStack(undefined)).toBe(false)
  })

  it('labels the tab and subtitle', () => {
    expect(stackTabAriaLabel(stack())).toBe('Stack, 2 branches')
    expect(
      stackTabAriaLabel(stack({ status: 'notStacked', branches: [] })),
    ).toBe('Stack, not stacked')
    expect(stackSubtitle(stack())).toBe('1 open branch stacked on main')
    expect(
      stackSubtitle(
        stack({ status: 'notStacked', currentBranch: 'main', branches: [] }),
      ),
    ).toBe('Add a branch to start a stack on main.')
    expect(commitsLabel(0)).toBe('No commits yet')
  })

  it('matches synced pull requests by repo and head branch', () => {
    const pr = { repoId: 'r1', head: 'b', num: 7 } as PullRequest
    const other = { repoId: 'r2', head: 'b', num: 8 } as PullRequest
    expect(pullRequestForBranch([other, pr], 'r1', 'b')?.num).toBe(7)
    expect(pullRequestForBranch([other], 'r1', 'b')).toBeUndefined()
  })
})
