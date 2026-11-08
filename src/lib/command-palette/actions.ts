import { resolveDefaultRepoId } from '$lib/settings/default-repo'
import { bangPromptWithExtra } from '$lib/bangs/bangs'
import { repos } from '$lib/state/repos.svelte'
import { settings } from '$lib/settings/store.svelte'
import { startThread } from '$lib/workspace/start-thread'

/**
 * Starts a thread from a bang in the default repo without leaving the current view.
 * `extra` is what was typed after `!name` in the palette, added to the prompt.
 */
export async function runBang(bangId: string, extra = '') {
  const bang = settings.bangs.find((row) => row.id === bangId)
  if (!bang) return
  await startThread({
    repoId: resolveDefaultRepoId(repos.items, settings.defaultRepo),
    prompt: bangPromptWithExtra(bang.prompt, extra),
  })
}
