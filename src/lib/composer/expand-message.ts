import { commands } from '$lib/ipc'
import type { ClickupTask } from '$lib/ipc/bindings'
import { clickup } from '$lib/state/clickup.svelte'
import { repos } from '$lib/state/repos.svelte'
import { settings } from '$lib/state/settings.svelte'
import { expandBangs } from '$lib/settings/bangs'
import { todos } from '$lib/state/todos.svelte'
import { todoAliasTask } from '$lib/todos/ref'
import { expandRepoAliases } from './repo-aliases'
import { expandTaskAliases, taskContext, todoContext } from './task-aliases'

/** How long sending waits for a task's description before going without it. */
const DESCRIPTION_WAIT_MS = 3000

async function description(task: ClickupTask): Promise<string> {
  const known = clickup.details[task.id]?.description
  if (known !== undefined) return known
  const fetched = commands
    .clickupTask(task.id)
    .then((result) => (result.status === 'ok' ? result.data.description : ''))
    .catch(() => '')
  const timeout = new Promise<string>((done) =>
    setTimeout(() => done(''), DESCRIPTION_WAIT_MS),
  )
  return Promise.race([fetched, timeout])
}

/**
 * What a message says to the agent: `!bang` as its prompt, `@repo` as the repo's folder,
 * and `#task` as the task's reference, name and link, with the mentioned tasks'
 * descriptions after it.
 */
export async function expandMessage(text: string): Promise<string> {
  const withBangs = expandBangs(text, settings.bangs)
  const withRepos = expandRepoAliases(withBangs, repos.items)
  const todoTasks = todos.items.map((todo) =>
    todoAliasTask(todo, todos.filesDir),
  )
  const { text: expanded, mentioned } = expandTaskAliases(withRepos, [
    ...todoTasks,
    ...(clickup.mine?.tasks ?? []),
  ])
  if (!mentioned.length) return expanded
  const sprint = mentioned.filter(
    (task): task is ClickupTask => !('kind' in task),
  )
  const descriptions = Object.fromEntries(
    await Promise.all(
      sprint.map(async (task) => [task.id, await description(task)]),
    ),
  )
  const context = [
    taskContext(sprint, descriptions),
    todoContext(mentioned.filter((task) => 'kind' in task)),
  ]
    .filter(Boolean)
    .join('\n\n')
  return context ? `${expanded}\n\n${context}` : expanded
}
