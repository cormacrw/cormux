<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import Layers from '@lucide/svelte/icons/layers'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import type { ToastPart, ToastTone } from '$lib/feedback/toast-payload'

  let {
    parts,
    tone = 'default' as ToastTone,
    onActivate,
  }: {
    parts: ToastPart[]
    tone?: ToastTone
    onActivate?: () => void
  } = $props()

  const Icon = $derived(
    tone === 'ok' ? CircleCheck : tone === 'bad' ? Trash2 : Layers,
  )
</script>

{#if onActivate}
  <button
    type="button"
    class="inline-flex w-full items-start gap-2 border-0 bg-transparent p-0 text-left text-sm leading-snug cursor-pointer"
    onclick={() => onActivate()}
  >
    <Icon class="mt-0.5 size-4 shrink-0 opacity-80" aria-hidden="true" />
    <span class="inline">
      {#each parts as part (part.type + part.value)}
        {#if part.type === 'code'}
          <code
            class="rounded bg-muted px-1 py-0.5 font-mono text-xs text-foreground"
            >{part.value}</code
          >
        {:else}
          {part.value}
        {/if}
      {/each}
    </span>
  </button>
{:else}
  <span class="inline-flex items-start gap-2 text-sm leading-snug">
    <Icon class="mt-0.5 size-4 shrink-0 opacity-80" aria-hidden="true" />
    <span class="inline">
      {#each parts as part (part.type + part.value)}
        {#if part.type === 'code'}
          <code
            class="rounded bg-muted px-1 py-0.5 font-mono text-xs text-foreground"
            >{part.value}</code
          >
        {:else}
          {part.value}
        {/if}
      {/each}
    </span>
  </span>
{/if}
