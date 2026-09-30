import { describe, expect, it } from 'vitest'
import { matchRepoId, parseOriginUrl } from './parse-origin'

describe('parseOriginUrl', () => {
  it('parses https remotes', () => {
    expect(parseOriginUrl('https://github.com/cormacrw/cormux.git')).toBe(
      'cormacrw/cormux',
    )
  })

  it('parses ssh remotes', () => {
    expect(parseOriginUrl('git@github.com:Acme/App.git')).toBe('acme/App')
  })
})

describe('matchRepoId', () => {
  it('matches registered repos case-insensitively', () => {
    expect(matchRepoId({ r1: 'cormacrw/cormux' }, 'CormacRW/Cormux')).toBe('r1')
  })
})
