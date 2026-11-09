import { describe, expect, it } from 'vitest'
import { entityLook } from './entities'

describe('entityLook', () => {
  it('reads GitHub pull requests and issues with their repo', () => {
    expect(entityLook('https://github.com/acme/app/pull/42')).toEqual({
      kind: 'pullRequest',
      source: 'acme/app',
    })
    expect(entityLook('https://github.com/acme/app/issues/7')).toEqual({
      kind: 'issue',
      source: 'acme/app',
    })
    expect(entityLook('https://github.com/acme/app')).toEqual({
      kind: 'link',
      source: 'acme/app',
    })
  })

  it('knows trackers and docs by host', () => {
    expect(entityLook('https://linear.app/acme/issue/ENG-118').kind).toBe(
      'ticket',
    )
    expect(entityLook('https://acme.atlassian.net/browse/OPS-3').kind).toBe(
      'ticket',
    )
    expect(entityLook('https://jira.acme.internal/browse/OPS-3').kind).toBe(
      'ticket',
    )
    expect(entityLook('https://docs.google.com/document/d/abc').kind).toBe(
      'doc',
    )
    expect(entityLook('https://www.figma.com/design/abc').kind).toBe('doc')
  })

  it('falls back to a plain link named by its host', () => {
    expect(entityLook('https://www.example.com/a')).toEqual({
      kind: 'link',
      source: 'example.com',
    })
    expect(entityLook('not a url')).toEqual({
      kind: 'link',
      source: 'not a url',
    })
  })
})
