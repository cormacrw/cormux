import { describe, expect, it } from 'vitest'
import {
  addRepoErrorMessage,
  removeRepoBlockReason,
  validateAddRepoPath,
} from './repos-validation'

describe('validateAddRepoPath', () => {
  const existing = [{ id: 'my-app', path: '~/code/my-app' }]

  it('accepts a normalized path', () => {
    expect(validateAddRepoPath('~/code/other/', existing)).toBeNull()
  })

  it('rejects empty and relative paths', () => {
    expect(validateAddRepoPath('', existing)).toBe('empty')
    expect(validateAddRepoPath('code/foo', existing)).toBe('invalid-path')
  })

  it('rejects duplicates', () => {
    expect(validateAddRepoPath('~/code/my-app', existing)).toBe(
      'duplicate-path',
    )
    expect(validateAddRepoPath('~/other/my-app', existing)).toBe(
      'duplicate-name',
    )
  })
})

describe('removeRepoBlockReason', () => {
  it('blocks last repo and in-use repos', () => {
    expect(
      removeRepoBlockReason({ id: 'only', name: 'only' }, 1, 0),
    ).toBe('Harness needs at least one repo')
    expect(
      removeRepoBlockReason({ id: 'a', name: 'a' }, 2, 2),
    ).toMatch(/Tear them down first/)
  })
})

describe('addRepoErrorMessage', () => {
  it('maps codes to copy', () => {
    expect(addRepoErrorMessage('duplicate-name', 'foo')).toContain('foo')
  })
})
