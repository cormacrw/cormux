import { taskRef } from '$lib/clickup/board'
import type { ClickupTask } from '$lib/ipc/bindings'

const MAX_SHOWN = 8
/** A long description is cut here, so one task can't swamp the message. */
const DESCRIPTION_LIMIT = 2000

type AliasTask = Pick<ClickupTask, 'id' | 'customId' | 'name' | 'url'>

/** A `#ref` word in the draft: where it starts and ends, and what's typed before the caret. */
export type TaskToken = { start: number; end: number; query: string }

/** The `#` word the caret is in, when it starts the draft or follows whitespace, or null. */
export function taskToken(draft: string, caret: number): TaskToken | null {
  const match = /(^|\s)#([\w-]*)$/.exec(draft.slice(0, caret))
  if (!match) return null
  const rest = /^\S*/.exec(draft.slice(caret))?.[0] ?? ''
  if (!/^[\w-]*$/.test(rest)) return null
  return {
    start: match.index + (match[1]?.length ?? 0),
    end: caret + rest.length,
    query: match[2] ?? '',
  }
}

/** The draft with `token` replaced by `#ref `, and where the caret goes after it. */
export function insertTaskAlias(
  draft: string,
  token: TaskToken,
  ref: string,
): { text: string; caret: number } {
  const rest = draft.slice(token.end)
  const inserted = `#${ref}${/^\s/.test(rest) ? '' : ' '}`
  return {
    text: draft.slice(0, token.start) + inserted + rest,
    caret: token.start + ref.length + 2,
  }
}

/** References starting with the query first, then names containing it. */
export function filterTasks<T extends AliasTask>(
  tasks: T[],
  query: string,
): T[] {
  const needle = query.toLowerCase()
  const starts = tasks.filter((task) =>
    taskRef(task).toLowerCase().startsWith(needle),
  )
  const contains = tasks.filter(
    (task) =>
      !starts.includes(task) && task.name.toLowerCase().includes(needle),
  )
  return [...starts, ...contains].slice(0, MAX_SHOWN)
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * The message with each `#ref` of a sprint task replaced by its reference, name and link, and
 * the tasks it mentioned, in order. Other `#` words (headings, PR numbers) stay as typed.
 */
export function expandTaskAliases<T extends AliasTask>(
  text: string,
  tasks: T[],
): { text: string; mentioned: T[] } {
  const byRef = new Map(
    tasks.map((task) => [taskRef(task).toLowerCase(), task]),
  )
  if (!byRef.size) return { text, mentioned: [] }
  const refs = [...byRef.keys()].sort((a, b) => b.length - a.length)
  const pattern = new RegExp(
    `(^|[\\s(])#(${refs.map(escapeRegExp).join('|')})(?![\\w-])`,
    'gi',
  )
  const mentioned: T[] = []
  const expanded = text.replace(pattern, (_, before: string, ref: string) => {
    const task = byRef.get(ref.toLowerCase())!
    if (!mentioned.includes(task)) mentioned.push(task)
    return `${before}${taskRef(task)} “${task.name}” (${task.url})`
  })
  return { text: expanded, mentioned }
}

/** The mentioned tasks' descriptions, for the agent, which can't read ClickUp itself. */
export function taskContext(
  mentioned: AliasTask[],
  descriptions: Record<string, string>,
): string {
  const blocks = mentioned.flatMap((task) => {
    const description = descriptions[task.id]?.trim()
    if (!description) return []
    const clipped =
      description.length > DESCRIPTION_LIMIT
        ? `${description.slice(0, DESCRIPTION_LIMIT).trimEnd()}…`
        : description
    return [`### ${taskRef(task)} ${task.name}\n${clipped}`]
  })
  return blocks.length ? `ClickUp tasks:\n\n${blocks.join('\n\n')}` : ''
}

/**
 * Where the mentioned TODOs are written down, for the agent to read: the core
 * keeps each one as `CMUX-<n>.md` (title, status, plan and notes).
 */
export function todoContext(mentioned: AliasTask[]): string {
  if (!mentioned.length) return ''
  const lines = mentioned.map(
    (task) => `- ${taskRef(task)} “${task.name}”: ${task.url}`,
  )
  return `Cormux TODOs. Read each file for the task's notes and status:\n${lines.join('\n')}`
}
