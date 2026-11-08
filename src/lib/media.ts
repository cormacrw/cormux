/** File kinds the app shows as images or videos, by lower-case extension. */
export const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp']
export const VIDEO_EXTENSIONS = ['mp4', 'webm', 'mov', 'm4v']

/** The lower-case extension of a path or URL, ignoring any query or fragment. */
export function extensionOf(source: string): string {
  const path = source.split(/[?#]/)[0] ?? ''
  const name = path.split('/').pop() ?? ''
  return name.includes('.') ? (name.split('.').pop()?.toLowerCase() ?? '') : ''
}
