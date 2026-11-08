import { MEDIA_EXTENSIONS, type MediaKind } from '$lib/ipc/bindings'

/** The lower-case extension of a path or URL, ignoring any query or fragment. */
export function extensionOf(source: string): string {
  const path = source.split(/[?#]/)[0] ?? ''
  const name = path.split('/').pop() ?? ''
  return name.includes('.') ? (name.split('.').pop()?.toLowerCase() ?? '') : ''
}

/** What the core shows a file as, by its extension: an image, a video, or neither. */
export function mediaKindOf(source: string): MediaKind | null {
  const extension = extensionOf(source)
  return Object.hasOwn(MEDIA_EXTENSIONS, extension)
    ? MEDIA_EXTENSIONS[extension as keyof typeof MEDIA_EXTENSIONS]
    : null
}
