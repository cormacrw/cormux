import { convertFileSrc } from '@tauri-apps/api/core'
import { commands, type ThreadMedia } from '$lib/ipc'
import type { LocalMediaKind } from './media'

export type ResolvedMedia = {
  kind: LocalMediaKind
  name: string
  /** A loadable URL: a data URL for small images, the asset protocol otherwise. */
  url: string
}

const resolvedCache = new Map<string, Promise<ResolvedMedia | null>>()

function toResolved(media: ThreadMedia): ResolvedMedia | null {
  const kind =
    media.kind === 'image' || media.kind === 'video' ? media.kind : null
  if (!kind) return null
  return {
    kind,
    name: media.name,
    url: media.previewUrl ?? convertFileSrc(media.path),
  }
}

/**
 * Asks the core to copy the referenced file into its media folder and say how
 * to show it. Cached per source and workdir, so re-renders and re-opens of a
 * thread resolve each file once. A null result means it isn't showable media.
 */
export function resolveLocalMedia(
  source: string,
  workdir: string | undefined,
): Promise<ResolvedMedia | null> {
  const key = `${workdir ?? ''}\0${source}`
  const cached = resolvedCache.get(key)
  if (cached) return cached
  const pending = commands
    .resolveThreadMedia(source, workdir ?? null)
    .then((result) =>
      result.status === 'ok' && result.data ? toResolved(result.data) : null,
    )
    .catch(() => null)
  resolvedCache.set(key, pending)
  return pending
}
