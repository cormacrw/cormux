import { describe, expect, it } from 'vitest'
import { showFindingsTab } from './tab'

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
        workspaceKind: 'lead',
        reviewReady: true,
        hasFindings: false,
      }),
    ).toBe(false)
    expect(
      showFindingsTab({
        workspaceKind: 'lead',
        reviewReady: false,
        hasFindings: true,
      }),
    ).toBe(true)
  })
})
