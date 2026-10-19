import { describe, expect, it } from 'vitest'
import {
  branchErrorMessage,
  validateBaseBranch,
  validateBranchName,
  workspaceBranchesForRepo,
} from './validation'

describe('workspaceBranchesForRepo', () => {
  it('only counts branches of the repo the workspace is for', () => {
    const records = [
      { repoId: 'web', branch: 'feat/login' },
      { repoId: 'api', branch: 'feat/billing' },
      { repoId: 'web', branch: 'fix/nav' },
    ]
    expect(workspaceBranchesForRepo(records, 'web')).toEqual([
      'feat/login',
      'fix/nav',
    ])
    expect(workspaceBranchesForRepo(records, 'docs')).toEqual([])
  })
})

describe('validateBranchName', () => {
  const ctx = {
    existingBranches: ['main', 'develop'],
    workspaceBranches: ['feat/active'],
  }

  it('requires a value', () => {
    expect(validateBranchName('', ctx)).toBe('required')
  })

  it('rejects invalid characters and paths', () => {
    expect(validateBranchName('feat//login', ctx)).toBe('format')
    expect(validateBranchName('/feat', ctx)).toBe('format')
    expect(validateBranchName('feat..x', ctx)).toBe('format')
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

describe('validateBaseBranch', () => {
  it('accepts known branches', () => {
    expect(validateBaseBranch('main', ['main', 'develop'])).toBeNull()
  })

  it('rejects unknown branches', () => {
    expect(validateBaseBranch('nope', ['main'])).toMatch(/No branch called/)
  })
})
