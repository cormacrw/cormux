<script lang="ts">
  import Folder from '@lucide/svelte/icons/folder'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import { Button } from '$lib/components/ui/button'
  import { commands } from '$lib/ipc'
  import type { RepoRecord } from '$lib/ipc/bindings'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { cn } from '$lib/utils'

  let { repo }: { repo: RepoRecord } = $props()

  let pulling = $state(false)

  const branch = $derived(repo.defaultBranch?.trim() || 'main')

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
    <span class="truncate font-mono text-xs text-muted-foreground"
      >{branch}</span
    >
  </span>
  <Button
    variant="ghost"
    size="icon"
    class="size-6 text-muted-foreground hover:text-foreground"
    aria-label="Pull {branch} for {repo.name}"
    title="Pull {branch}"
    disabled={pulling}
    onclick={() => void pull()}
  >
    <RefreshCw
      class={cn('size-3.5', pulling && 'animate-spin')}
      aria-hidden="true"
    />
  </Button>
</li>
