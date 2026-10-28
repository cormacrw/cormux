import { convertFileSrc } from '@tauri-apps/api/core'
import { commands, type ThreadMedia } from '$lib/ipc'

/**
 * Media an agent shares by referencing a local file in its reply, the way a
 * screenshot tool leaves it: `![after](/tmp/shot.png)`, `[the flow](demo.webm)`,
 * or a raw `<img>`/`<video>` tag. Remote sources are not media; the core copies
 * local ones into its own media folder before the webview may load them.
 */
export type LocalMediaKind = 'image' | 'video'

export type MediaSegment =
  | { kind: 'text'; text: string }
  | { kind: 'media'; source: string; alt: string; mediaKind: LocalMediaKind }

const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp']
const VIDEO_EXTENSIONS = ['mp4', 'webm', 'mov', 'm4v']

function extensionOf(source: string): string {
  const path = source.split(/[?#]/)[0] ?? ''
  return path.split('.').pop()?.toLowerCase() ?? ''
}

/** Local means no remote scheme: an absolute path, a relative path, or a file URL. */
export function isLocalSource(source: string): boolean {
  const trimmed = source.trim()
  if (!trimmed) return false
  if (trimmed.startsWith('file://')) return true
  if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) return false
  return true
}

export function mediaKindForSource(source: string): LocalMediaKind | null {
  if (!isLocalSource(source)) return null
  const extension = extensionOf(source)
  if (IMAGE_EXTENSIONS.includes(extension)) return 'image'
  if (VIDEO_EXTENSIONS.includes(extension)) return 'video'
  return null
}

// One pass over the reply: markdown images, markdown links, and raw media tags.
// A source is either angle-bracketed (so it may contain spaces) or a bare run.
const MEDIA_PATTERN =
  /!\[([^\]]*)\]\(\s*(?:<([^>]+)>|([^\s)]+))(?:\s+["'][^"']*["'])?\s*\)|(?<!!)\[([^\]]+)\]\(\s*(?:<([^>]+)>|([^\s)]+))(?:\s+["'][^"']*["'])?\s*\)|<img\b[^>]*?\bsrc\s*=\s*["']([^"']+)["'][^>]*>|<video\b[^>]*?\bsrc\s*=\s*["']([^"']+)["'][^>]*>(?:\s*<\/video>)?/gi

function attr(tag: string, name: string): string {
  const match = new RegExp(`\\b${name}\\s*=\\s*["']([^"']*)["']`, 'i').exec(tag)
  return match?.[1] ?? ''
}

function splitPlain(text: string, into: MediaSegment[]) {
  let last = 0
  for (const match of text.matchAll(MEDIA_PATTERN)) {
    const source =
      match[2] ?? match[3] ?? match[5] ?? match[6] ?? match[7] ?? match[8] ?? ''
    const mediaKind = mediaKindForSource(source)
    if (!mediaKind) continue
    const alt =
      match[1] ??
      match[4] ??
      (match[7] !== undefined ? attr(match[0], 'alt') : '')
    if (match.index > last) {
      into.push({ kind: 'text', text: text.slice(last, match.index) })
    }
    into.push({ kind: 'media', source, alt: alt.trim(), mediaKind })
    last = match.index + match[0].length
  }
  if (last < text.length) into.push({ kind: 'text', text: text.slice(last) })
}

/** Splits a reply into prose and local media, keeping order. Fenced code stays prose. */
export function splitLocalMedia(text: string): MediaSegment[] {
  const segments: MediaSegment[] = []
  const fence = /```[^\n]*\n[\s\S]*?(?:```|$)/g
  let last = 0
  for (const match of text.matchAll(fence)) {
    splitPlain(text.slice(last, match.index), segments)
    segments.push({ kind: 'text', text: match[0] })
    last = match.index + match[0].length
  }
  splitPlain(text.slice(last), segments)
  // A match that produced nothing (all references left as prose) stays one text segment.
  if (segments.every((segment) => segment.kind === 'text')) {
    return text ? [{ kind: 'text', text }] : []
  }
  return segments.filter(
    (segment) => segment.kind !== 'text' || segment.text.length > 0,
  )
}

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
