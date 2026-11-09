<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import X from '@lucide/svelte/icons/x'
  import {
    runSucceeded,
    runSummaryLabel,
    type RunOutput,
  } from '$lib/thread/reply/blocks/run'

  let { data }: { data: RunOutput } = $props()

  const summary = $derived(data.summary)
  const ok = $derived(runSucceeded(summary))
</script>

<details class="reply-card group/run" data-od-id="reply-run">
  <summary class="reply-card-head cursor-pointer select-none">
    {#if ok}
      <Check class="size-3.5 shrink-0 text-success" aria-hidden="true" />
    {:else}
      <X class="size-3.5 shrink-0 text-destructive" aria-hidden="true" />
    {/if}
    <span class="font-medium text-foreground">{runSummaryLabel(summary)}</span>
    <span class="ml-auto shrink-0">Output</span>
  </summary>
  <pre
    class="max-h-72 overflow-auto border-t border-border px-3 py-2 font-mono text-(length:--code-font-size) whitespace-pre-wrap">{data.output}</pre>
</details>
{#if summary.failures.length}
  <ul class="reply-run-failures">
    {#each summary.failures as failure (failure)}
      <li class="truncate font-mono" title={failure}>{failure}</li>
    {/each}
  </ul>
{/if}
