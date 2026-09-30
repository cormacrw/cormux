import { describe, expect, it } from 'vitest'
import {
  branchErrorMessage,
  validateBaseBranch,
  validateBranchName,
} from './validation'

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
})

describe('validateBaseBranch', () => {
  it('accepts known branches', () => {
    expect(validateBaseBranch('main', ['main', 'develop'])).toBeNull()
  })

  it('rejects unknown branches', () => {
    expect(validateBaseBranch('nope', ['main'])).toMatch(/No branch called/)
  })
})
