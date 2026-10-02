<script lang="ts">
  import { onDestroy } from 'svelte'
  import { taskRef } from '$lib/clickup/board'
  import type { ClickupTask } from '$lib/ipc/bindings'
  import { cn } from '$lib/utils'
  import Check from '@lucide/svelte/icons/check'
  import Copy from '@lucide/svelte/icons/copy'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import { openUrl } from '@tauri-apps/plugin-opener'

  let {
    task,
    class: className,
  }: {
    task: Pick<ClickupTask, 'id' | 'customId' | 'url'>
    class?: string
  } = $props()

  const ref = $derived(taskRef(task))
  let copied = $state(false)
  let timer: ReturnType<typeof setTimeout> | undefined

  async function copy() {
    try {
      await navigator.clipboard.writeText(ref)
    } catch {
      return
    }
    copied = true
    clearTimeout(timer)
    timer = setTimeout(() => (copied = false), 1500)
  }

  onDestroy(() => clearTimeout(timer))

  const button =
    'inline-flex size-6 items-center justify-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50'
</script>

<!-- Inside a draggable card: these buttons must not start a drag or select the card. -->
<div class={cn('flex items-center gap-0.5', className)} data-card-action>
  <button
    type="button"
    class={button}
    aria-label={copied ? `Copied ${ref}` : `Copy ${ref}`}
    title={copied ? 'Copied' : `Copy ${ref}`}
    onclick={(event) => {
      event.stopPropagation()
      void copy()
    }}
  >
    {#if copied}
      <Check class="size-3.5 text-success" aria-hidden="true" />
    {:else}
      <Copy class="size-3.5" aria-hidden="true" />
    {/if}
  </button>
  <button
    type="button"
    class={button}
    aria-label="Open {ref} in ClickUp"
    title="Open in ClickUp"
    onclick={(event) => {
      event.stopPropagation()
      void openUrl(task.url)
    }}
  >
    <ExternalLink class="size-3.5" aria-hidden="true" />
  </button>
</div>
