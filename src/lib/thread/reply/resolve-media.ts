import { convertFileSrc } from '@tauri-apps/api/core'
import { commands, type ThreadMedia } from '$lib/ipc'
import type { MediaKind } from '$lib/ipc/bindings'

export type ResolvedMedia = {
  kind: MediaKind
  name: string
  /** A loadable URL: a data URL for small images, the asset protocol otherwise. */
  url: string
}

const resolvedCache = new Map<string, Promise<ResolvedMedia | null>>()

function toResolved(media: ThreadMedia): ResolvedMedia {
  return {
    kind: media.kind,
    name: media.name,
    url: media.previewUrl ?? convertFileSrc(media.path),
  }
}

/**
 * Asks the core to copy the referenced file into its media folder and say how
 * to show it. A file it found is cached per source and workdir, so re-renders and
 * re-opens of a thread resolve it once. A null result means it isn't showable media
 * (yet: it's asked again next time), and a failure is logged rather than kept.
 */
export function resolveLocalMedia(
  source: string,
  workdir: string | undefined,
): Promise<ResolvedMedia | null> {
  const key = `${workdir ?? ''}\0${source}`
  const cached = resolvedCache.get(key)
  if (cached) return cached
  const forget = () => {
    if (resolvedCache.get(key) === pending) resolvedCache.delete(key)
  }
  const pending = commands
    .resolveThreadMedia(source, workdir ?? null)
    .then((result) => {
      if (result.status === 'error') {
        console.warn(`could not resolve media ${source}`, result.error)
        return null
      }
      return result.data ? toResolved(result.data) : null
    })
    .catch((error: unknown) => {
      console.warn(`could not resolve media ${source}`, error)
      return null
    })
    .then((resolved) => {
      if (!resolved) forget()
      return resolved
    })
  resolvedCache.set(key, pending)
  return pending
}
