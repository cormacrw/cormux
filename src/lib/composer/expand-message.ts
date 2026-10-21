import { commands } from '$lib/ipc'
import type { ClickupTask } from '$lib/ipc/bindings'
import { clickup } from '$lib/state/clickup.svelte'
import { repos } from '$lib/state/repos.svelte'
import { expandRepoAliases } from './repo-aliases'
import { expandTaskAliases, taskContext } from './task-aliases'

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
 * What a message says to the agent: `@repo` as the repo's folder, and `#task` as the task's
 * reference, name and link, with the mentioned tasks' descriptions after it.
 */
export async function expandMessage(text: string): Promise<string> {
  const withRepos = expandRepoAliases(text, repos.items)
  const { text: expanded, mentioned } = expandTaskAliases(
    withRepos,
    clickup.mine?.tasks ?? [],
  )
  if (!mentioned.length) return expanded
  const descriptions = Object.fromEntries(
    await Promise.all(
      mentioned.map(async (task) => [task.id, await description(task)]),
    ),
  )
  const context = taskContext(mentioned, descriptions)
  return context ? `${expanded}\n\n${context}` : expanded
}
