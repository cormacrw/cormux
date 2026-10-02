<script lang="ts">
  import Folder from '@lucide/svelte/icons/folder'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import { Button } from '$lib/components/ui/button'
  import { commands } from '$lib/ipc'
  import type { RepoRecord } from '$lib/ipc/bindings'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { repos } from '$lib/state'
  import { plural } from '$lib/sidebar/status'
  import { cn } from '$lib/utils'

  let { repo }: { repo: RepoRecord } = $props()

  let pulling = $state(false)

  const branch = $derived(repo.defaultBranch?.trim() || 'main')
  // Commits the local default branch is behind and ahead of origin, as of the last fetch.
  const git = $derived(repos.gitById[repo.id])
  const behind = $derived(git?.behind ?? 0)
  const ahead = $derived(git?.ahead ?? 0)
  const gitLabel = $derived.by(() => {
    const parts = []
    if (behind) parts.push(`${behind} behind`)
    if (ahead) parts.push(`${ahead} ahead of`)
    return parts.length ? `${parts.join(', ')} origin/${branch}` : ''
  })

  async function pull() {
    if (pulling) return
    pulling = true
    const result = await commands.pullRepoDefaultBranch(repo.id)
    pulling = false
    if (result.status === 'error') toastCoreError(result.error)
  }
</script>

<li
  class="grid grid-cols-[16px_minmax(0,1fr)_auto] items-center gap-2 rounded-md py-1 pl-2 pr-1"
  data-od-id="side-repo-{repo.id}"
>
  <Folder class="size-3.5 text-muted-foreground" aria-hidden="true" />
  <span class="min-w-0 grid">
    <span class="truncate text-sm text-sidebar-foreground">{repo.name}</span>
    <span
      class="flex min-w-0 items-center gap-1.5 font-mono text-xs text-muted-foreground"
    >
      <span class="truncate">{branch}</span>
      {#if behind || ahead}
        <span
          class="shrink-0"
          title={gitLabel}
          aria-hidden="true"
          data-od-id="side-repo-git-{repo.id}"
        >
          {#if behind}<span class="text-amber-600 dark:text-amber-400"
              >↓{behind}</span
            >{/if}{#if behind && ahead}&nbsp;{/if}{#if ahead}<span
              class="text-sky-600 dark:text-sky-400">↑{ahead}</span
            >{/if}
        </span>
        <span class="sr-only">{gitLabel}</span>
      {/if}
    </span>
  </span>
  <Button
    variant="ghost"
    size="icon"
    class="size-6 text-muted-foreground hover:text-foreground"
    aria-label="Pull {branch} for {repo.name}"
    title={behind
      ? `Pull ${plural(behind, 'commit')} into ${branch}`
      : `Pull ${branch}`}
    disabled={pulling}
    onclick={() => void pull()}
  >
    <RefreshCw
      class={cn('size-3.5', pulling && 'animate-spin')}
      aria-hidden="true"
    />
  </Button>
</li>
