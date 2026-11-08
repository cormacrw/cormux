<script lang="ts">
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import type { DiffBlock } from '$lib/thread/reply/blocks/diff'
  import { revealInChanges } from '$lib/nav/open'

  let { data: diff, workdir }: { data: DiffBlock; workdir?: string } = $props()
</script>

<figure class="reply-card" data-od-id="reply-diff">
  <figcaption class="reply-card-head">
    <span class="truncate font-mono">{diff.path ?? 'Diff'}</span>
    <span class="shrink-0 font-mono">
      <span class="text-success">+{diff.added}</span>
      <span class="text-destructive">−{diff.deleted}</span>
    </span>
    {#if diff.path && workdir}
      <button
        type="button"
        class="ml-auto inline-flex shrink-0 items-center gap-0.5 font-medium text-foreground/80 hover:text-foreground"
        onclick={() => revealInChanges(diff.path!)}
      >
        Open in Changes
        <ArrowRight class="size-3" aria-hidden="true" />
      </button>
    {/if}
  </figcaption>
  <div class="reply-diff">
    {#each diff.lines as line, index (index)}
      <div class="reply-diff-line" data-line={line.kind}>
        {line.text || ' '}
      </div>
    {/each}
  </div>
</figure>
