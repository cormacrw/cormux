import { describe, expect, it } from 'vitest'
import { draftBranchName, draftWorkspaceName } from './draft'

describe('draftWorkspaceName', () => {
  it('capitalises and takes whole words up to 40 characters', () => {
    const prompt =
      'lazy-load the chart widgets on /dashboard so first paint does not wait'
    expect(draftWorkspaceName(prompt)).toBe('Lazy-load the chart widgets on')
  })

  it('returns empty for blank prompt', () => {
    expect(draftWorkspaceName('   ')).toBe('')
  })
})

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
