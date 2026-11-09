import { getContext, setContext } from 'svelte'

const MEDIA_HOST = Symbol('media-host')

type Host = () => HTMLElement | undefined

/**
 * For a view that shows conversations, as it starts: the element a lightbox opened in one of
 * them covers. It has to sit outside the timeline, whose view-transition stacking context
 * would trap the overlay under the view's header.
 */
export function provideMediaHost(host: Host) {
  setContext(MEDIA_HOST, host)
}

/** For a lightbox, as it starts: where it mounts, from the view it's in. */
export function mediaHost(): Host {
  return getContext<Host | undefined>(MEDIA_HOST) ?? (() => undefined)
}
