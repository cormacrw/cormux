<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import ReplyMarkdown from '../reply/ReplyMarkdown.svelte'
  import type { RowContext } from './context'
  import Fold from './Fold.svelte'
  import RowShell from './RowShell.svelte'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'thought' }>; ctx: RowContext } =
    $props()
</script>

<RowShell id={item.id} {ctx}>
  <Fold
    id={item.id}
    {ctx}
    class="group/thought max-w-[42rem]"
    summaryClass="inline-flex cursor-pointer list-none items-center gap-1 text-xs text-muted-foreground select-none hover:text-foreground [&::-webkit-details-marker]:hidden"
  >
    {#snippet summary()}
      <ChevronRight
        class="size-3 transition-transform group-open/thought:rotate-90"
        aria-hidden="true"
      />
      {#if item.id === ctx.activeIds.thought}
        <span class="animate-pulse">Thinking…</span>
      {:else}
        Thought
      {/if}
    {/snippet}
    <div
      class="mt-1 border-l-2 border-border pl-3 text-sm text-muted-foreground"
    >
      <ReplyMarkdown
        text={item.text}
        workdir={ctx.workdir}
        findPr={ctx.findPr}
      />
    </div>
  </Fold>
</RowShell>
