<script lang="ts">
  import { CountUp } from '$lib/components/ui/count-up'
  import { motionMs } from '$lib/motion'
  import {
    formatChangeCounts,
    type DiffLineTotals,
  } from '$lib/workspace/diff-totals'
  import { untrack } from 'svelte'

  let { totals }: { totals: DiffLineTotals } = $props()

  // Each count springs from the last totals to the new ones, so a commit
  // counts down to zero and the label falls away once it lands.
  let from = $state(untrack(() => ({ ...totals })))
  let to = $state(untrack(() => ({ ...totals })))

  $effect(() => {
    const { added, deleted } = totals
    untrack(() => {
      if (added === to.added && deleted === to.deleted) return
      from = { ...to }
      to = { added, deleted }
    })
  })

  const animated = $derived(motionMs(1) > 0)
  const staticLabel = $derived(formatChangeCounts(totals))
</script>

{#snippet count(sign: string, key: keyof DiffLineTotals)}
  {#if to[key] > 0 || from[key] > 0}
    <span
      >{sign}<CountUp
        from={from[key]}
        to={to[key]}
        duration={1}
        onEnd={() => (from[key] = to[key])}
      /></span
    >
  {/if}
{/snippet}

{#if animated}
  {#if to.added > 0 || to.deleted > 0 || from.added > 0 || from.deleted > 0}
    <span
      class="flex gap-1 font-mono text-[10px] text-muted-foreground tabular-nums"
      aria-hidden="true"
    >
      {@render count('+', 'added')}
      {@render count('−', 'deleted')}
    </span>
  {/if}
{:else if staticLabel}
  <span class="font-mono text-[10px] text-muted-foreground" aria-hidden="true">
    {staticLabel}
  </span>
{/if}
