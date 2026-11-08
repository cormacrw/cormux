import { describe, expect, it } from 'vitest'
import { routeIsLive, windowTitle, type Route } from './route'

const lookups = {
  workspace: (id: string) => id === 'ws-1',
  scratch: (id: string) => id === 'scratch-1',
  repo: (id: string) => id === 'my-app',
  sprint: true,
}

describe('routeIsLive', () => {
  it('keeps a route whose workspace, scratch or repo still exists', () => {
    const routes: Route[] = [
      { view: 'workspace', workspaceId: 'ws-1', threadId: null },
      { view: 'scratch', scratchId: 'scratch-1' },
      { view: 'repo', repoId: 'my-app' },
      { view: 'sprint' },
      { view: 'settings', section: 'repos' },
      { view: 'newThread' },
    ]
    for (const route of routes) expect(routeIsLive(route, lookups)).toBe(true)
  })

  it('drops a route once the thing it shows is gone', () => {
    const gone: Route[] = [
      { view: 'workspace', workspaceId: 'ws-torn-down', threadId: 'th-1' },
      { view: 'scratch', scratchId: 'scratch-ended' },
      { view: 'repo', repoId: 'removed' },
    ]
    for (const route of gone) expect(routeIsLive(route, lookups)).toBe(false)
  })

  it('drops the sprint once ClickUp is disconnected', () => {
    expect(routeIsLive({ view: 'sprint' }, { ...lookups, sprint: false })).toBe(
      false,
    )
  })
})

describe('windowTitle', () => {
  it('titles the new thread page and Settings', () => {
    expect(windowTitle({ view: 'newThread' }, undefined)).toBe(
      'Cormux · New thread',
    )
    expect(windowTitle({ view: 'settings', section: 'repos' }, undefined)).toBe(
      'Cormux · Settings',
    )
  })

  it('names dev builds in the title', () => {
    expect(windowTitle({ view: 'sprint' }, undefined, 'Cormux Dev')).toBe(
      'Cormux Dev · Sprint',
    )
  })

  it('titles an open workspace, scratch or repo by name, else by id', () => {
    const workspace: Route = {
      view: 'workspace',
      workspaceId: 'ws-1',
      threadId: null,
    }
    expect(windowTitle(workspace, 'Auth session timeout')).toBe(
      'Cormux · Auth session timeout',
    )
    expect(windowTitle(workspace, undefined)).toBe('Cormux · ws-1')
    expect(
      windowTitle({ view: 'scratch', scratchId: 's-1' }, 'Webhook check'),
    ).toBe('Cormux · Webhook check')
    expect(windowTitle({ view: 'repo', repoId: 'my-app' }, 'my-app')).toBe(
      'Cormux · my-app',
    )
  })
})
