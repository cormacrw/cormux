import { tick } from 'svelte'
import { NEW_THREAD, type Route, type View } from './route'

type RouteOf<V extends View> = Extract<Route, { view: V }>

class Nav {
  route = $state.raw<Route>(NEW_THREAD)
  private listeners = new Set<(to: Route, from: Route) => void>()
  private arrivals = new Set<() => void>()

  get view(): View {
    return this.route.view
  }
  get workspaceId(): string | null {
    return this.route.view === 'workspace' ? this.route.workspaceId : null
  }
  get threadId(): string | null {
    return this.route.view === 'workspace' ? this.route.threadId : null
  }
  get scratchId(): string | null {
    return this.route.view === 'scratch' ? this.route.scratchId : null
  }
  get repoId(): string | null {
    return this.route.view === 'repo' ? this.route.repoId : null
  }

  navigate(route: Route) {
    const from = this.route
    this.route = route
    for (const listener of this.listeners) listener(route, from)
    for (const arrival of this.arrivals) arrival()
  }

  /** Runs on every navigation, before views react; for stores that follow the route. */
  onNavigate(listener: (to: Route, from: Route) => void) {
    this.listeners.add(listener)
    return () => void this.listeners.delete(listener)
  }

  /**
   * Runs `arrive` now and again after every navigation, even to the same route, once the views
   * have drawn it: for a view moving focus on arrival. Call it in an `$effect`, which it returns
   * the stop for; what `arrive` reads now re-runs it as the effect's own dependencies.
   */
  onArrival(arrive: () => void) {
    arrive()
    const later = () =>
      void tick().then(() => {
        if (this.arrivals.has(later)) arrive()
      })
    this.arrivals.add(later)
    return () => void this.arrivals.delete(later)
  }

  /** The route's one-shot `key` when it's on `view`, cleared so it isn't taken twice. */
  take<V extends View, K extends Exclude<keyof RouteOf<V>, 'view'>>(
    view: V,
    key: K,
  ): RouteOf<V>[K] | undefined {
    const route = this.route
    if (route.view !== view) return undefined
    const value = (route as RouteOf<V>)[key]
    if (value === undefined) return undefined
    this.route = { ...route, [key]: undefined }
    return value
  }
}

export const nav = new Nav()

export function navigate(route: Route) {
  nav.navigate(route)
}
