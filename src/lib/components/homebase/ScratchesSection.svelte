<script lang="ts">
  import ScratchCard from '$lib/components/scratch/ScratchCard.svelte'
  import { scratches } from '$lib/state'
</script>

<section
  class="grid gap-4"
  aria-labelledby="sess-section-title"
  data-od-id="home-sessions"
>
  <h2
    id="sess-section-title"
    class="flex items-center gap-2 text-sm font-medium tracking-tight"
  >
    Scratches
    <span
      class="rounded-full border border-border bg-muted/50 px-2 py-0.5 font-mono text-xs text-muted-foreground"
      >{scratches.items.length}</span
    >
  </h2>

  {#if scratches.items.length === 0}
    <p
      class="rounded-xl border border-dashed border-border/80 px-4 py-4 text-sm text-muted-foreground"
    >
      No scratches. Start one for a question that doesn’t need its own branch.
    </p>
  {:else}
    <div
      class="grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(min(100%,320px),1fr))]"
    >
      {#each scratches.items as scratch (scratch.id)}
        <ScratchCard {scratch} />
      {/each}
    </div>
  {/if}
</section>
