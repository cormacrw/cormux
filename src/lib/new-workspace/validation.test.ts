import { describe, expect, it } from 'vitest'
import { branchErrorMessage, validateBranchName } from './validation'

describe('validateBranchName', () => {
  const ctx = {
    existingBranches: ['main', 'develop'],
    workspaceBranches: ['feat/active'],
    validRef: true,
  }

  it('requires a value', () => {
    expect(validateBranchName('', ctx)).toBe('required')
    expect(validateBranchName('  ', { ...ctx, validRef: false })).toBe(
      'required',
    )
  })

  it('takes the core’s word on what git accepts', () => {
    expect(validateBranchName('feat//login', { ...ctx, validRef: false })).toBe(
      'format',
    )
    expect(validateBranchName('feat/é', ctx)).toBeNull()
  })

  it('detects existing branches in the selected repo', () => {
    expect(validateBranchName('main', ctx)).toBe('exists')
    expect(branchErrorMessage('main', 'exists')).toContain(
      'main already exists',
    )
  })

  it('detects workspace branch conflicts', () => {
    expect(validateBranchName('feat/active', ctx)).toBe('workspaceConflict')
  })

  it("rejects the repo's default branch", () => {
    expect(
      validateBranchName('develop', { ...ctx, defaultBranch: 'develop' }),
    ).toBe('default')
  })
})
