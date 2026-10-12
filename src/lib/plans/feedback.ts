/** A note on a stretch of the rendered plan, which `start`/`end` locate as text offsets. */
export type PlanComment = {
  id: string
  start: number
  end: number
  /** The highlighted text, so the agent knows what the note is about. */
  quote: string
  body: string
}

const QUOTE_LIMIT = 400

function quoteLines(text: string): string {
  const flat = text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .join('\n')
  const clipped =
    flat.length > QUOTE_LIMIT
      ? `${flat.slice(0, QUOTE_LIMIT).trimEnd()}…`
      : flat
  return clipped
    .split('\n')
    .map((line) => `> ${line}`)
    .join('\n')
}

/** One message for the agent: each comment as the quoted plan text, then the note, in plan order. */
export function formatPlanFeedback(
  comments: PlanComment[],
  note: string,
): string {
  const blocks = [...comments]
    .sort((a, b) => a.start - b.start)
    .map((comment) => `${quoteLines(comment.quote)}\n${comment.body.trim()}`)
  const overall = note.trim()
  if (overall) blocks.push(blocks.length ? `Overall: ${overall}` : overall)
  return `I reviewed your plan. Revise it with this feedback, then present the updated plan. Don't start editing yet.\n\n${blocks.join('\n\n')}`
}
