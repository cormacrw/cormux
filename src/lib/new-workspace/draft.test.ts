import { describe, expect, it } from 'vitest'
import { draftBranchName, fallbackBranchName, uniqueBranchName } from './draft'

describe('draftBranchName', () => {
  it('uses feat prefix for feature work', () => {
    expect(draftBranchName('Implement OAuth login with Supabase')).toBe(
      'feat/implement-oauth-login-with-supabase',
    )
  })

  it('uses fix prefix when prompt mentions bugs', () => {
    expect(draftBranchName('Fix crash on logout')).toBe(
      'fix/fix-crash-on-logout',
    )
  })

  it('uses refactor prefix for refactors', () => {
    expect(draftBranchName('Refactor auth module')).toBe(
      'refactor/refactor-auth-module',
    )
  })
})

describe('blank prompt fallbacks', () => {
  it('picks the next free feat/workspace branch', () => {
    expect(fallbackBranchName([])).toBe('feat/workspace')
    expect(fallbackBranchName(['feat/workspace', 'feat/workspace-2'])).toBe(
      'feat/workspace-3',
    )
  })
})

describe('uniqueBranchName', () => {
  it('numbers a taken name', () => {
    expect(uniqueBranchName('feat/a', [])).toBe('feat/a')
    expect(uniqueBranchName('feat/a', ['feat/a', 'feat/a-2'])).toBe('feat/a-3')
  })
})
