<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import Copy from '@lucide/svelte/icons/copy'
  import { onDestroy } from 'svelte'

  let { getText }: { getText: () => string } = $props()

  let copied = $state(false)
  let copiedTimer: ReturnType<typeof setTimeout> | undefined

  async function copy() {
    try {
      await navigator.clipboard.writeText(getText())
    } catch {
      return
    }
    copied = true
    clearTimeout(copiedTimer)
    copiedTimer = setTimeout(() => (copied = false), 1500)
  }

  onDestroy(() => clearTimeout(copiedTimer))
</script>

<button
  type="button"
  class="code-copy absolute top-1.5 right-1.5 inline-flex size-6 items-center justify-center rounded bg-muted text-foreground/60 opacity-0 transition hover:text-foreground focus-visible:opacity-100"
  aria-label="Copy code"
  title="Copy code"
  onclick={copy}
>
  {#if copied}
    <Check class="size-3.5" aria-hidden="true" />
  {:else}
    <Copy class="size-3.5" aria-hidden="true" />
  {/if}
</button>
