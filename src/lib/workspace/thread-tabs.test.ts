import { describe, expect, it } from 'vitest'
import {
  buildThreadBarTabOrder,
  findingsTabAriaLabel,
  moveTabFocusIndex,
  outputTabAriaLabel,
  outputTabState,
  showFindingsTab,
  threadTabAriaLabel,
} from './thread-tabs'

const sampleThread = {
  id: 't1',
  workspaceId: 'ws1',
  role: 'Lead',
  engine: 'claude',
  status: 'running',
  paused: false,
  activity: 'Editing files',
  pendingApprovals: 0,
}

describe('thread tab labels', () => {
  it('formats thread aria labels with approvals', () => {
    expect(threadTabAriaLabel(sampleThread)).toBe('Lead, Editing files')
    expect(threadTabAriaLabel({ ...sampleThread, pendingApprovals: 2 })).toBe(
      'Lead, Editing files, 2 approvals waiting',
    )
  })

  it('formats findings and output labels', () => {
    expect(findingsTabAriaLabel(3)).toBe('Review findings, 3 open findings')
    expect(
      outputTabAriaLabel({
        provisioning: false,
        appStatus: 'running',
        port: 5173,
      }),
    ).toBe('App output, running on localhost:5173')
    expect(
      outputTabState({
        provisioning: true,
        appStatus: 'stopped',
        port: null,
      }),
    ).toBe('setting up')
  })
})

describe('thread bar order', () => {
  it('puts findings after threads, then changes and stack before output', () => {
    const tabs = buildThreadBarTabOrder([sampleThread], true)
    expect(tabs.map((tab) => tab.kind)).toEqual([
      'thread',
      'findings',
      'changes',
      'stack',
      'output',
    ])
  })

  it('shows findings tab when review is ready', () => {
    expect(
      showFindingsTab({ workspaceKind: 'review', reviewReady: true }),
    ).toBe(true)
    expect(
      showFindingsTab({ workspaceKind: 'review', reviewReady: false }),
    ).toBe(false)
    expect(showFindingsTab({ workspaceKind: null, reviewReady: true })).toBe(
      false,
    )
  })
})

describe('keyboard wrap', () => {
  it('wraps arrow navigation', () => {
    expect(moveTabFocusIndex(2, 3, 'ArrowRight')).toBe(0)
    expect(moveTabFocusIndex(0, 3, 'ArrowLeft')).toBe(2)
    expect(moveTabFocusIndex(1, 4, 'Home')).toBe(0)
    expect(moveTabFocusIndex(1, 4, 'End')).toBe(3)
  })
})
