import type { Snapshot, StateChanged } from '$lib/ipc'
import { app } from './app.svelte'

export { app } from './app.svelte'
export { prs } from './prs.svelte'
export { settings } from './settings.svelte'
export { threads } from './threads.svelte'
export { workspaces } from './workspaces.svelte'

export function hydrateFromSnapshot(snapshot: Snapshot) {
  app.hydrate(snapshot)
}

export function patchFromEvent(event: StateChanged) {
  app.version = event.version
}
