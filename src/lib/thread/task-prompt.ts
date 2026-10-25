import type { WeekItem } from '$lib/week/items'

/** Settings › Prompts › Task prompt, when it's left as is. */
export const DEFAULT_TASK_PROMPT =
  'Complete {task}: read the task, make the change, and check it works.'

/**
 * What picking a task types into the composer: the template with `{task}`
 * filled in, or the task added after it if the template doesn't say where.
 * Both kinds go in as `#ref` (a ClickUp ID, or a TODO's `CMUX-<n>`), which
 * sending expands into the task's name and where to read the rest.
 */
export function taskPrompt(
  item: Pick<WeekItem, 'tag'>,
  template = DEFAULT_TASK_PROMPT,
): string {
  const task = `#${item.tag}`
  const base = template.trim() || DEFAULT_TASK_PROMPT
  return base.includes('{task}')
    ? base.replaceAll('{task}', task)
    : `${base} ${task}`
}
