import type { Component } from 'svelte'
import type { View } from '$lib/core/route'
import NewThreadView from '../new-thread/NewThreadView.svelte'
import WorkspaceView from '../workspace/WorkspaceView.svelte'
import { loadView, SHELL_DIALOGS, type Loader } from './lazy-views'

/**
 * What each route view shows. The first screen's are bundled; everything else loads on its
 * first visit, so startup only parses what it paints.
 */
export const ROUTE_VIEWS: Record<
  View,
  { component: Component } | { load: Loader }
> = {
  newThread: { component: NewThreadView },
  workspace: { component: WorkspaceView },
  settings: { load: () => import('../../settings-page/SettingsView.svelte') },
  repo: { load: () => import('../repo/RepoView.svelte') },
  scratch: { load: () => import('../scratch/ScratchView.svelte') },
  sprint: { load: () => import('../../clickup/components/SprintView.svelte') },
}

/** Fetches every lazy view and dialog once the app has settled, so the first click doesn't land on a blank frame. */
export function preloadLazyViews() {
  const views = Object.values(ROUTE_VIEWS).flatMap((view) =>
    'load' in view ? [view.load] : [],
  )
  for (const load of [...views, ...Object.values(SHELL_DIALOGS)]) {
    void loadView(load, (error) =>
      console.warn('Could not preload a view', error),
    )
  }
}
