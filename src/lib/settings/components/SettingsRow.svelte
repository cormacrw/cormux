<script lang="ts">
  import type { Snippet } from 'svelte'
  import { cn } from '$lib/utils'

  let {
    title,
    description,
    controlId,
    class: className,
    control,
  }: {
    title: string
    description?: string
    controlId?: string
    class?: string
    control: Snippet
  } = $props()
</script>

<div
  class={cn(
    'flex flex-col gap-3 border-b border-border px-4 py-3 last:border-b-0 sm:flex-row sm:items-center sm:justify-between',
    className,
  )}
>
  <div class="min-w-0 flex-1 space-y-0.5">
    {#if controlId}
      <label class="text-sm font-medium" for={controlId}>{title}</label>
    {:else}
      <p class="text-sm font-medium">{title}</p>
    {/if}
    {#if description}
      <p class="text-xs text-muted-foreground">{description}</p>
    {/if}
  </div>
  <div class="shrink-0">{@render control()}</div>
</div>
