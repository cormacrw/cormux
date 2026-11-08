import { describe, expect, it } from 'vitest'
import {
  routeIsLive,
  windowTitle,
  type Route,
  type RouteLookups,
} from './route'

const named =
  (names: Record<string, string>) =>
  (id: string): string | undefined =>
    names[id]

const lookups: RouteLookups = {
  workspace: named({ 'ws-1': 'Auth session timeout' }),
  scratch: named({ 'scratch-1': 'Webhook check' }),
  repo: named({ 'my-app': 'my-app' }),
  sprint: true,
}

const nothing: RouteLookups = {
  workspace: () => undefined,
  scratch: () => undefined,
  repo: () => undefined,
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
    expect(windowTitle({ view: 'newThread' }, lookups)).toBe(
      'Cormux · New thread',
    )
    expect(windowTitle({ view: 'settings', section: 'repos' }, lookups)).toBe(
      'Cormux · Settings',
    )
  })

  it('names dev builds in the title', () => {
    expect(windowTitle({ view: 'sprint' }, lookups, 'Cormux Dev')).toBe(
      'Cormux Dev · Sprint',
    )
  })

  it('titles an open workspace, scratch or repo by name, else by id', () => {
    const workspace: Route = {
      view: 'workspace',
      workspaceId: 'ws-1',
      threadId: null,
    }
    expect(windowTitle(workspace, lookups)).toBe(
      'Cormux · Auth session timeout',
    )
    expect(windowTitle(workspace, nothing)).toBe('Cormux · ws-1')
    expect(
      windowTitle({ view: 'scratch', scratchId: 'scratch-1' }, lookups),
    ).toBe('Cormux · Webhook check')
    expect(windowTitle({ view: 'repo', repoId: 'my-app' }, lookups)).toBe(
      'Cormux · my-app',
    )
  })
})
