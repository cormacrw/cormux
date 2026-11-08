import { extensionOf, IMAGE_EXTENSIONS, VIDEO_EXTENSIONS } from '$lib/media'

/**
 * Media an agent shares by referencing a local file in its reply, the way a
 * screenshot tool leaves it: `![after](/tmp/shot.png)`, `[the flow](demo.webm)`,
 * or a raw `<img>`/`<video>` tag. Remote sources are not media; the core copies
 * local ones into its own media folder before the webview may load them.
 */
export type LocalMediaKind = 'image' | 'video'

export type MediaReference = {
  /** The reference as written. */
  raw: string
  source: string
  alt: string
  mediaKind: LocalMediaKind
}

/** Marks a local image the sanitiser let through; its `src` is never loaded directly. */
export const INLINE_MEDIA_ATTR = 'data-local-media'

/** Local means no remote scheme: an absolute path, a relative path, or a file URL. */
function isLocalSource(source: string): boolean {
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

// One pass over the text: markdown images, markdown links, and raw media tags.
// A source is either angle-bracketed (so it may contain spaces) or a bare run.
const MEDIA_PATTERN =
  /!\[([^\]]*)\]\(\s*(?:<([^>]+)>|([^\s)]+))(?:\s+["'][^"']*["'])?\s*\)|(?<!!)\[([^\]]+)\]\(\s*(?:<([^>]+)>|([^\s)]+))(?:\s+["'][^"']*["'])?\s*\)|<img\b[^>]*?\bsrc\s*=\s*["']([^"']+)["'][^>]*>|<video\b[^>]*?\bsrc\s*=\s*["']([^"']+)["'][^>]*>(?:\s*<\/video>)?/gi

function attr(tag: string, name: string): string {
  const match = new RegExp(`\\b${name}\\s*=\\s*["']([^"']*)["']`, 'i').exec(tag)
  return match?.[1] ?? ''
}

/** Text with its local media references cut out, in order. */
export function splitMedia(text: string): (string | MediaReference)[] {
  const parts: (string | MediaReference)[] = []
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
    if (match.index > last) parts.push(text.slice(last, match.index))
    parts.push({ raw: match[0], source, alt: alt.trim(), mediaKind })
    last = match.index + match[0].length
  }
  if (last < text.length) parts.push(text.slice(last))
  return parts
}

/** Two images in a row whose captions say before and after become one comparison. */
export function isBeforeAfter(
  first: MediaReference,
  second: MediaReference,
): boolean {
  return (
    first.mediaKind === 'image' &&
    second.mediaKind === 'image' &&
    /\bbefore\b/i.test(first.alt) &&
    /\bafter\b/i.test(second.alt)
  )
}
