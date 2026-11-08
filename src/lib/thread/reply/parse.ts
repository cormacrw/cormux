import { Marked, type Token } from 'marked'
import { memoByText } from '$lib/memo'
import { isBeforeAfter, splitMedia, type MediaReference } from './media'
import { BLOCKS, type ReplyBlock } from './registry'
import { findTagBlock } from './tag-block'

/** A reply as it renders, in order. `raw` is each part's markdown source. */
export type ReplySegment =
  /** Markdown, with its top-level blocks (`blocks`) so a reply being written renders each once. */
  | { kind: 'text'; raw: string; blocks: string[] }
  | ({ kind: 'media' } & MediaReference)
  | { kind: 'compare'; raw: string; before: string; after: string }
  | { kind: 'block'; raw: string; block: ReplyBlock }
  /** Data the agent appended in tags; it shows after the prose. */
  | { kind: 'tagBlock'; raw: string; block: ReplyBlock }

type ParseOptions = {
  /**
   * The reply is still being written, so a block that isn't finished is held back: the last
   * top-level block stays markdown, and an unclosed tag block isn't shown at all.
   */
  streaming: boolean
}

const marked = new Marked({ gfm: true, breaks: true })

// Only these top-level tokens are cut around their media. Cutting a table, list or quote would
// leave the markdown on either side unparseable, so media in those stays in the prose and is
// mounted in place once rendered.
const SPLITTABLE = new Set(['paragraph', 'html', 'space', 'text'])

function tokenBlock(token: Token): ReplyBlock | null {
  for (const def of BLOCKS) {
    if ('hidden' in def || !('token' in def.match)) continue
    const body = def.match.token(token)
    if (body === null) continue
    const data = def.parse(body)
    if (data !== null) return { kind: def.kind, data } as ReplyBlock
  }
  return null
}

function addText(segments: ReplySegment[], raw: string, isBlock: boolean) {
  if (!raw) return
  const blocks = isBlock && raw.trim() ? [raw] : []
  const previous = segments.at(-1)
  if (previous?.kind === 'text') {
    previous.raw += raw
    previous.blocks.push(...blocks)
  } else {
    segments.push({ kind: 'text', raw, blocks })
  }
}

function pairBeforeAfter(segments: ReplySegment[]): ReplySegment[] {
  const paired: ReplySegment[] = []
  for (let at = 0; at < segments.length; at += 1) {
    const first = segments[at]!
    const gap = segments[at + 1]
    const nextAt = gap?.kind === 'text' && !gap.raw.trim() ? at + 2 : at + 1
    const second = segments[nextAt]
    if (
      first.kind === 'media' &&
      second?.kind === 'media' &&
      isBeforeAfter(first, second)
    ) {
      paired.push({
        kind: 'compare',
        raw: segments
          .slice(at, nextAt + 1)
          .map((segment) => segment.raw)
          .join(''),
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

/**
 * Markdown split at what renders as components: fenced and math blocks, local media and
 * before/after pairs. Only top-level blocks count, so a fence inside a list stays in the list.
 * Tag blocks aren't looked for; see `parseReply`.
 */
function parseMarkdown(text: string, streaming: boolean): ReplySegment[] {
  const tokens = marked.lexer(text)
  const growing = tokens.findLastIndex((token) => token.type !== 'space')
  const segments: ReplySegment[] = []
  tokens.forEach((token, index) => {
    const block = streaming && index === growing ? null : tokenBlock(token)
    if (block) {
      segments.push({ kind: 'block', raw: token.raw, block })
    } else if (SPLITTABLE.has(token.type)) {
      for (const part of splitMedia(token.raw)) {
        if (typeof part !== 'string') segments.push({ kind: 'media', ...part })
        else addText(segments, part, token.type !== 'space')
      }
    } else {
      addText(segments, token.raw, true)
    }
  })
  return pairBeforeAfter(segments)
}

const finishedMarkdown = memoByText((text) => parseMarkdown(text, false), 300)

/** `parseReply` for text already without its tag blocks, such as a reply's prose as it types. */
export function parseProse(
  text: string,
  { streaming }: ParseOptions,
): ReplySegment[] {
  return streaming ? parseMarkdown(text, true) : finishedMarkdown(text)
}

function parse(text: string, streaming: boolean): ReplySegment[] {
  let prose = text
  const tagged: ReplySegment[] = []
  for (const def of BLOCKS) {
    if (!('tag' in def.match)) continue
    const found = findTagBlock(prose, def.match.tag, streaming)
    if (!found) continue
    if (found.kind === 'unfinished') {
      prose = prose.slice(0, found.start).trimEnd()
      continue
    }
    if (!('hidden' in def)) {
      const data = def.parse(found.body)
      if (data === null) continue
      tagged.push({
        kind: 'tagBlock',
        raw: prose.slice(found.start, found.end),
        block: { kind: def.kind, data } as ReplyBlock,
      })
    }
    prose = (prose.slice(0, found.start) + prose.slice(found.end)).trimEnd()
  }
  return [...parseMarkdown(prose, streaming), ...tagged]
}

const finishedReplies = memoByText((text) => parse(text, false), 300)
const streamingReplies = memoByText((text) => parse(text, true), 20)

/**
 * A reply split into what it renders as: prose, block components, local media, and the tag
 * blocks the agent appended (hidden ones, like the Reviewer's findings, left out).
 */
export function parseReply(
  text: string,
  { streaming }: ParseOptions,
): ReplySegment[] {
  return streaming ? streamingReplies(text) : finishedReplies(text)
}

/** The reply's markdown without its tag blocks: what types out, and what Copy copies. */
export function replyProse(segments: ReplySegment[]): string {
  return segments
    .filter((segment) => segment.kind !== 'tagBlock')
    .map((segment) => segment.raw)
    .join('')
    .trim()
}

/** What a reader can see of a reply, for ⌘F: its prose and the text of its tag blocks. */
export function replySearchText(segments: ReplySegment[]): string {
  return segments
    .map((segment) => {
      if (segment.kind !== 'tagBlock') return segment.raw
      const def = BLOCKS.find((entry) => entry.kind === segment.block.kind)
      return def && 'text' in def && def.text
        ? (def.text as (data: unknown) => string)(segment.block.data)
        : ''
    })
    .join('\n')
}
