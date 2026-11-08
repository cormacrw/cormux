import { describe, expect, it } from 'vitest'
import type { PullRequest } from './prs.svelte'
import { prByNumber, prForBranch } from './pr-lookup'

const pr = { repoId: 'my-app', head: 'feat/colors', num: 12 } as PullRequest

describe('prForBranch', () => {
  it('matches a pull request on this repo by head branch', () => {
    expect(prForBranch([pr], 'my-app', 'feat/colors')).toBe(pr)
    expect(prForBranch([pr], 'other', 'feat/colors')).toBeUndefined()
    expect(prForBranch([pr], 'my-app', 'main')).toBeUndefined()
  })
})

describe('prByNumber', () => {
  it('matches a pull request on this repo by number', () => {
    expect(prByNumber([pr], 'my-app', 12)).toBe(pr)
    expect(prByNumber([pr], 'other', 12)).toBeUndefined()
  })
})
