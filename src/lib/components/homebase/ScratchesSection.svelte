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
    class="clay-section"
  >
    Scratches · {scratches.items.length}
  </h2>

  {#if scratches.items.length === 0}
    <p
      class="rounded-[22px_18px_20px_16px] bg-card px-5 py-4 text-[15px] font-semibold text-card-foreground shadow-lift-1"
    >
      No scratches. Start one for a question that doesn’t need its own branch.
    </p>
  {:else}
    <div
      class="grid gap-4 [grid-template-columns:repeat(auto-fill,minmax(min(100%,300px),1fr))]"
    >
      {#each scratches.items as scratch (scratch.id)}
        <ScratchCard {scratch} />
      {/each}
    </div>
  {/if}
</section>
