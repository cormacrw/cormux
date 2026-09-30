import { describe, expect, it } from 'vitest'
import {
  filterPullRequests,
  matchesPrFilter,
  prFilterCounts,
  type PullRequestRow,
} from './pr-filter'

const sample: PullRequestRow[] = [
  { id: '1', rel: 'review' },
  { id: '2', rel: 'author' },
  { id: '3', rel: 'mention' },
]

describe('matchesPrFilter', () => {
  it('shows all relationships under all', () => {
    expect(matchesPrFilter('mention', 'all')).toBe(true)
  })

  it('limits review segment to review requests', () => {
    expect(matchesPrFilter('review', 'review')).toBe(true)
    expect(matchesPrFilter('author', 'review')).toBe(false)
  })

  it('limits yours segment to authored PRs', () => {
    expect(matchesPrFilter('author', 'author')).toBe(true)
    expect(matchesPrFilter('review', 'author')).toBe(false)
  })
})

describe('prFilterCounts', () => {
  it('counts segments for the toggle labels', () => {
    expect(prFilterCounts(sample)).toEqual({ all: 3, review: 1, author: 1 })
  })
})

describe('filterPullRequests', () => {
  it('returns only matching rows', () => {
    expect(filterPullRequests(sample, 'review')).toEqual([
      { id: '1', rel: 'review' },
    ])
  })
})
