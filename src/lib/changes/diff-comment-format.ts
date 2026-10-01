export type DiffCommentSide = 'old' | 'new'

export type DiffComment = {
  id: string
  path: string
  side: DiffCommentSide
  line: number
  /** The line's code when the comment was written, so the message survives later edits. */
  code: string
  body: string
}

/**
 * One message for the agent: each comment as `path:line`, the quoted code, then the note.
 * `branch` names the branch when the comments aren't on the one the agent has checked out.
 */
export function formatCommentsForAgent(
  comments: DiffComment[],
  branch?: string,
): string {
  const sorted = [...comments].sort(
    (a, b) => a.path.localeCompare(b.path) || a.line - b.line,
  )
  const blocks = sorted.map((comment) => {
    const where = `${comment.path}:${comment.line}${comment.side === 'old' ? ' (removed line)' : ''}`
    const code = comment.code.trimEnd()
    const quote = code ? `\n> ${code}` : ''
    return `${where}${quote}\n${comment.body.trim()}`
  })
  const on = branch ? `your changes on ${branch}` : 'your changes'
  return `Review comments on ${on}:\n\n${blocks.join('\n\n')}`
}
