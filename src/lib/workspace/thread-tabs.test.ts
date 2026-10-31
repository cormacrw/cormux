import { describe, expect, it } from 'vitest'
import {
  findingsTabAriaLabel,
  outputTabAriaLabel,
  outputTabState,
  showFindingsTab,
} from './thread-tabs'

describe('thread tab labels', () => {
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

describe('findings', () => {
  it('shows findings tab when review is ready', () => {
    expect(
      showFindingsTab({
        workspaceKind: 'review',
        reviewReady: true,
        hasFindings: false,
      }),
    ).toBe(true)
    expect(
      showFindingsTab({
        workspaceKind: 'review',
        reviewReady: false,
        hasFindings: true,
      }),
    ).toBe(false)
  })

  it('shows findings tab on a regular workspace once a review found something', () => {
    expect(
      showFindingsTab({
        workspaceKind: null,
        reviewReady: true,
        hasFindings: false,
      }),
    ).toBe(false)
    expect(
      showFindingsTab({
        workspaceKind: null,
        reviewReady: false,
        hasFindings: true,
      }),
    ).toBe(true)
  })
})
