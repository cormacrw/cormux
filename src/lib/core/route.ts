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

/** The views that open one thing, which the route names by id. */
type Subject = 'workspace' | 'scratch' | 'repo'

/** The other views are pages, always the same place, titled by these. */
const PAGE_TITLES: Record<Exclude<View, Subject>, string> = {
  newThread: 'New thread',
  settings: 'Settings',
  sprint: 'Sprint',
}

/**
 * Names the one thing a route opens, by its view; undefined once it's gone. The stores answer,
 * so this module imports none.
 */
export type RouteLookups = Record<
  Subject,
  (id: string) => string | undefined
> & {
  /** The Sprint page only exists while ClickUp is connected. */
  sprint: boolean
}

/** What the route has open: one thing by view and id, else a page by title. */
function routeTarget(
  route: Route,
): { view: Subject; id: string } | { page: string } {
  switch (route.view) {
    case 'workspace':
      return { view: route.view, id: route.workspaceId }
    case 'scratch':
      return { view: route.view, id: route.scratchId }
    case 'repo':
      return { view: route.view, id: route.repoId }
    default:
      return { page: PAGE_TITLES[route.view] }
  }
}

/** Whether the place a route names still exists; when it doesn't, the app falls back to a new thread. */
export function routeIsLive(route: Route, live: RouteLookups): boolean {
  if (route.view === 'sprint') return live.sprint
  const target = routeTarget(route)
  return 'page' in target || live[target.view](target.id) !== undefined
}

/** The window title: the app, then where it is, naming the open workspace, scratch or repo. */
export function windowTitle(
  route: Route,
  live: RouteLookups,
  appName = 'Cormux',
): string {
  const target = routeTarget(route)
  const where =
    'page' in target ? target.page : (live[target.view](target.id) ?? target.id)
  return `${appName} · ${where}`
}
