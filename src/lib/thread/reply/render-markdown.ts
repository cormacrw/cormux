import { markdownRenderer } from '$lib/markdown/render'
import { INLINE_MEDIA_ATTR, mediaKindForSource } from './media'

/**
 * Replies and thoughts. A local image (the kind a top-level reference renders as a figure) is
 * kept for `mountInlineMedia`, which resolves it through the core.
 */
export const replyMarkdown = markdownRenderer({
  attr: INLINE_MEDIA_ATTR,
  keep: (source) => mediaKindForSource(source) === 'image',
})
