// How an agent's reply types out in the thread. Claude sends a whole reply at once, so typing
// is only a reveal: it should feel typed for a short reply but never leave the reader waiting
// on a long one the agent finished long ago.

/** The typed feel for a short reply. */
const BASE_CHARS_PER_SEC = 90

/** However far behind, the reveal catches up within this long. */
const CATCH_UP_SEC = { live: 1.2, done: 0.4 }

/** A word this close to the reveal is shown whole, rather than letter by letter. */
const WORD_REACH = 14

/** Characters per second to reveal at, given how much is left and whether the agent is still writing. */
export function revealRate(remaining: number, live: boolean): number {
  const catchUp = live ? CATCH_UP_SEC.live : CATCH_UP_SEC.done
  return Math.max(BASE_CHARS_PER_SEC, remaining / catchUp)
}

/** Where the reveal lands after `step` more characters, finishing the word it stops in. */
export function revealEnd(text: string, shown: number, step: number): number {
  const target = Math.min(text.length, shown + Math.max(1, step))
  if (target >= text.length) return text.length
  const rest = text.slice(target, target + WORD_REACH)
  const space = rest.search(/\s/)
  return space === -1 ? target : target + space
}

function countOutsideCode(text: string, marker: string): number {
  // Markers inside inline code don't open or close anything.
  const withoutCode = text.replace(/`[^`\n]*`/g, '')
  return withoutCode.split(marker).length - 1
}

/**
 * Where a table still missing its header or `|---|` row starts, or null. Until both lines are
 * written it can't parse as a table, so it would show as raw pipes.
 */
function unfinishedTableStart(text: string): number | null {
  const lines = text.split('\n')
  // A trailing newline leaves an empty last line: the row before it is the one to look at.
  let first = lines.at(-1) === '' ? lines.length - 1 : lines.length
  const end = first
  while (first > 0 && /^ {0,3}\|/.test(lines[first - 1]!)) first -= 1
  const finishedLines = lines.length - 1 - first
  if (first === end || finishedLines >= 2) return null
  return lines.slice(0, first).join('\n').length + (first > 0 ? 1 : 0)
}

/**
 * Half of a reply as markdown that renders the way the finished reply will: an unclosed code
 * fence, bold or inline code is closed for now, and a link still being written is held back,
 * and so is a table until its header row is done, so formatting doesn't flicker in as raw symbols
 * and then jump.
 */
export function stableMarkdown(partial: string): string {
  const fences = partial.match(/^```/gm)?.length ?? 0
  if (fences % 2 === 1) {
    // Inside a code block: show it as a block straight away.
    return partial.endsWith('\n') ? `${partial}\`\`\`` : `${partial}\n\`\`\``
  }
  const table = unfinishedTableStart(partial)
  let text = table === null ? partial : partial.slice(0, table)
  // A link whose target isn't finished yet, `[text](http…`, waits until it is.
  const link = /\[[^\]\n]*(\]\([^)\s]*)?$/.exec(text)
  if (link) text = text.slice(0, link.index)
  const lastLine = text.slice(text.lastIndexOf('\n') + 1)
  // Inline code that's still open on the last line.
  if ((lastLine.split('`').length - 1) % 2 === 1) text += '`'
  // Bold that's still open on the last line (`**` and `__`).
  for (const marker of ['**', '__']) {
    const line = text.slice(text.lastIndexOf('\n') + 1)
    if (countOutsideCode(line, marker) % 2 === 1) {
      text = text.replace(/\s+$/, '') + marker
    }
  }
  return text
}
