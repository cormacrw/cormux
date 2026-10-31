import type { MediaSegment } from './local-media'

export type CompareSegment = { kind: 'compare'; before: string; after: string }

const isGap = (segment: MediaSegment | undefined) =>
  segment?.kind === 'text' && !segment.text.trim()

/** Two images in a row whose captions say before and after become one comparison. */
export function pairBeforeAfter(
  segments: MediaSegment[],
): (MediaSegment | CompareSegment)[] {
  const paired: (MediaSegment | CompareSegment)[] = []
  for (let at = 0; at < segments.length; at += 1) {
    const first = segments[at]!
    const nextAt = isGap(segments[at + 1]) ? at + 2 : at + 1
    const second = segments[nextAt]
    if (
      first.kind === 'media' &&
      second?.kind === 'media' &&
      first.mediaKind === 'image' &&
      second.mediaKind === 'image' &&
      /\bbefore\b/i.test(first.alt) &&
      /\bafter\b/i.test(second.alt)
    ) {
      paired.push({
        kind: 'compare',
        before: first.source,
        after: second.source,
      })
      at = nextAt
    } else {
      paired.push(first)
    }
  }
  return paired
}
