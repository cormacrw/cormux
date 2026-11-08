/** A workspace panel a route can open on; `findings` also moves focus to its heading. */
export type WorkspacePanel = 'thread' | 'findings' | 'changes' | 'output'

/** In Settings › Repositories, which field of the repo to focus. */
export type RepoField = 'runCommand' | 'setup'

/** A section of Settings a route can scroll to (the page lists them in `settings-page/sections.ts`). */
export type SettingsSectionId =
  | 'appearance'
  | 'general'
  | 'agents'
  | 'prompts'
  | 'github'
  | 'clickup'
  | 'repos'
  | 'bangs'
  | 'notifications'
  | 'shortcuts'
  | 'experimental'

/**
 * Where the app is. Optional fields are one-shot intents: the view they open takes each once
 * (`nav.take`), so coming back to the same place later doesn't repeat them.
 */
export type Route =
  | { view: 'newThread' }
  | {
      view: 'workspace'
      workspaceId: string
      threadId: string | null
      panel?: WorkspacePanel
      /** A file to expand and scroll to in Changes, at a line of its new side when given. */
      reveal?: { path: string; line: number | null }
    }
  | { view: 'scratch'; scratchId: string }
  | {
      view: 'settings'
      section?: SettingsSectionId
      /** A repo to expand in Settings › Repositories, focusing `field` when given. */
      repoId?: string
      field?: RepoField
    }
  | { view: 'sprint' }
  | { view: 'repo'; repoId: string }

export type View = Route['view']

export const NEW_THREAD: Route = { view: 'newThread' }

/** What a route needs to still make sense; the stores answer, so this module imports none. */
type RouteLookups = {
  workspace: (id: string) => boolean
  scratch: (id: string) => boolean
  repo: (id: string) => boolean
  sprint: boolean
}

/** Whether the place a route names still exists; when it doesn't, the app falls back to a new thread. */
export function routeIsLive(route: Route, live: RouteLookups): boolean {
  switch (route.view) {
    case 'workspace':
      return live.workspace(route.workspaceId)
    case 'scratch':
      return live.scratch(route.scratchId)
    case 'repo':
      return live.repo(route.repoId)
    case 'sprint':
      return live.sprint
    default:
      return true
  }
}

/** The id of what the route has open, for places that are one thing. */
export function routeSubject(route: Route): string | null {
  switch (route.view) {
    case 'workspace':
      return route.workspaceId
    case 'scratch':
      return route.scratchId
    case 'repo':
      return route.repoId
    default:
      return null
  }
}

/** The window title: the app, then where it is; `name` names the open workspace, scratch or repo. */
export function windowTitle(
  route: Route,
  name: string | undefined,
  appName = 'Cormux',
): string {
  if (route.view === 'settings') return `${appName} · Settings`
  if (route.view === 'sprint') return `${appName} · Sprint`
  const subject = routeSubject(route)
  if (subject) return `${appName} · ${name ?? subject}`
  return `${appName} · New thread`
}
