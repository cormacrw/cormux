import { taskRef } from '$lib/clickup/board'
import type { ClickupTask } from '$lib/ipc/bindings'
import { escapeRegExp, prefixFirst } from './match'

/** A long description is cut here, so one task can't swamp the message. */
const DESCRIPTION_LIMIT = 2000

type AliasTask = Pick<ClickupTask, 'id' | 'customId' | 'name' | 'url'>

/** References starting with the query first, then names containing it. */
export function filterTasks<T extends AliasTask>(
  tasks: T[],
  query: string,
): T[] {
  return prefixFirst(
    tasks,
    query,
    (task, needle) => taskRef(task).toLowerCase().startsWith(needle),
    (task, needle) => task.name.toLowerCase().includes(needle),
  )
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
