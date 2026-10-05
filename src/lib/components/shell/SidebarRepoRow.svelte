<script lang="ts">
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import { Button } from '$lib/components/ui/button'
  import { commands } from '$lib/ipc'
  import type { RepoRecord } from '$lib/ipc/bindings'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { repos } from '$lib/state'
  import { plural } from '$lib/sidebar/status'
  import { cn } from '$lib/utils'
  import { clayColor } from '$lib/clay/identity'
  import Buddy from '$lib/components/clay/Buddy.svelte'

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
  class="group grid grid-cols-[30px_minmax(0,1fr)_auto] items-center gap-3 rounded-[20px_24px_18px_22px] py-2 pr-1 pl-3 text-sidebar-foreground"
  data-od-id="side-repo-{repo.id}"
>
  <Buddy color={clayColor(repo.id)} face="none" size={30} />
  <span class="min-w-0 grid">
    <span class="truncate text-[16px] leading-tight font-bold">{repo.name}</span>
    <span
      class="flex min-w-0 items-center gap-1.5 text-[13.5px] leading-tight opacity-80"
    >
      <span class="truncate">{branch}</span>
      {#if behind || ahead}
        <span
          class="shrink-0 font-mono text-[12px] font-semibold"
          title={gitLabel}
          aria-hidden="true"
          data-od-id="side-repo-git-{repo.id}"
        >
          {#if behind}<span class="text-custard">↓{behind}</span
            >{/if}{#if behind && ahead}&nbsp;{/if}{#if ahead}<span
              class="text-[#a9cdeb]">↑{ahead}</span
            >{/if}
        </span>
        <span class="sr-only">{gitLabel}</span>
      {/if}
    </span>
  </span>
  <Button
    variant="ghost"
    size="icon"
    class="size-8 text-sidebar-foreground/80 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-white/10 hover:text-sidebar-foreground {behind
      ? 'opacity-100'
      : ''}"
    aria-label="Pull {branch} for {repo.name}"
    title={behind
      ? `Pull ${plural(behind, 'commit')} into ${branch}`
      : `Pull ${branch}`}
    disabled={pulling}
    onclick={() => void pull()}
  >
    <RefreshCw
      class={cn('size-3.5', pulling && 'animate-spin')}
      strokeWidth={2.5}
      aria-hidden="true"
    />
  </Button>
</li>
