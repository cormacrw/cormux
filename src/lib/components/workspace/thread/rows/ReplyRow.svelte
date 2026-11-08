<script lang="ts">
  import { onDestroy } from 'svelte'
  import Check from '@lucide/svelte/icons/check'
  import Copy from '@lucide/svelte/icons/copy'
  import { CopyFeedback } from '$lib/primitives/copy-feedback.svelte'
  import { parseReply, replyProse } from '$lib/thread/reply/parse'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import ReplyBlock from '../reply/ReplyBlock.svelte'
  import StreamingText from '../StreamingText.svelte'
  import type { RowContext } from './context'
  import RowShell from './RowShell.svelte'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'reply' }>; ctx: RowContext } =
    $props()

  const live = $derived(item.id === ctx.activeIds.reply)
  const reply = $derived(parseReply(item.text, { streaming: live }))
  const replyText = $derived(replyProse(reply))
  const clipboard = new CopyFeedback()
  onDestroy(() => clipboard.dispose())
</script>

<RowShell id={item.id} {ctx}>
  <!-- A reply that was only a findings block has nothing to show. -->
  <div class={['group max-w-[42rem]', !reply.length && 'hidden']}>
    {#if replyText}
      <div class="text-sm text-foreground">
        <StreamingText
          text={replyText}
          entries={ctx.entries}
          key={item.id}
          {live}
          workdir={ctx.workdir}
          findPr={ctx.findPr}
        />
      </div>
    {/if}
    {#each reply as segment, index (index)}
      {#if segment.kind === 'tagBlock'}
        <ReplyBlock block={segment.block} workdir={ctx.workdir} />
      {/if}
    {/each}
    <!-- Under the reply, not over it, so it never covers the last words of a line. -->
    <div class="mt-1 flex h-5 items-center">
      <button
        type="button"
        class="inline-flex size-5 items-center justify-center rounded text-muted-foreground opacity-0 transition hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
        aria-label="Copy message"
        title="Copy message"
        onclick={() => clipboard.copy(replyText)}
      >
        {#if clipboard.copied}
          <Check class="size-3" aria-hidden="true" />
        {:else}
          <Copy class="size-3" aria-hidden="true" />
        {/if}
      </button>
    </div>
  </div>
</RowShell>
