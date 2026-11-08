import { describe, expect, it } from 'vitest'
import { removeRepoBlockReason } from './repos-validation'

describe('removeRepoBlockReason', () => {
  it('blocks last repo and in-use repos', () => {
    expect(removeRepoBlockReason({ id: 'only', name: 'only' }, 1, 0)).toBe(
      'Harness needs at least one repo',
    )
    expect(removeRepoBlockReason({ id: 'a', name: 'a' }, 2, 2)).toMatch(
      /Tear them down first/,
    )
  })
})
