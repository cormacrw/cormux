<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import Layers from '@lucide/svelte/icons/layers'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import type { ToastPart, ToastTone } from '$lib/feedback/toast-payload'
  import { cn } from '$lib/utils'

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

  // Solid fills so the tone reads at a glance; same colours in light and dark.
  const toneClass = $derived(
    tone === 'ok'
      ? 'border-emerald-700/40 bg-emerald-600 text-white'
      : tone === 'bad'
        ? 'border-red-700/40 bg-red-600 text-white'
        : 'border-amber-500/50 bg-amber-400 text-amber-950',
  )
  const toastClass = $derived(
    cn(
      'flex w-[356px] max-w-[calc(100vw-2rem)] items-start gap-2 rounded-lg border px-4 py-3 text-left text-sm leading-snug shadow-lg',
      toneClass,
    ),
  )
</script>

{#snippet content()}
  <Icon class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
  <span class="inline">
    {#each parts as part (part.type + part.value)}
      {#if part.type === 'code'}
        <code class="rounded bg-black/15 px-1 py-0.5 font-mono text-xs"
          >{part.value}</code
        >
      {:else}
        {part.value}
      {/if}
    {/each}
  </span>
{/snippet}

{#if onActivate}
  <button
    type="button"
    class={cn(
      toastClass,
      'cursor-pointer transition-[filter] hover:brightness-105',
    )}
    data-tone={tone}
    onclick={() => onActivate()}
  >
    {@render content()}
  </button>
{:else}
  <div class={toastClass} data-tone={tone}>
    {@render content()}
  </div>
{/if}
