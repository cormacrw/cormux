<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import { editedPaths, toolRunSummary } from '$lib/thread/tool-run-summary'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import { revealInChanges } from '$lib/workspace/open'
  import type { RowContext } from './context'
  import Fold from './Fold.svelte'
  import RowShell from './RowShell.svelte'
  import RunSteps from './RunSteps.svelte'
  import { runCollapses } from './tool-run'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'toolRun' }>; ctx: RowContext } =
    $props()

  const collapsed = $derived(
    runCollapses(item.steps, item.id === ctx.activeIds.run),
  )
</script>

<RowShell id={item.id} {ctx}>
  {#if collapsed}
    {@const edited = editedPaths(item.steps)}
    <Fold
      id={item.id}
      {ctx}
      class="group/run"
      odId="tool-run-summary"
      summaryClass="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground select-none hover:text-foreground"
    >
      {#snippet summary()}
        <ChevronRight
          class="size-3 shrink-0 transition-transform group-open/run:rotate-90"
          aria-hidden="true"
        />
        <span class="truncate">{toolRunSummary(item.steps)}</span>
      {/snippet}
      <div class="mt-1.5 border-l-2 border-border pl-3">
        <RunSteps steps={item.steps} {ctx} />
      </div>
    </Fold>
    {#if edited.length}
      <ul class="mt-1.5 flex flex-wrap gap-1" data-od-id="files-touched">
        {#each edited as path (path)}
          {@const file = ctx.diffFile(path)}
          <li>
            <button
              type="button"
              class="file-ref"
              title={path}
              aria-label="View diff for {path}"
              onclick={() => revealInChanges(path)}
            >
              {path.split('/').pop()}
              {#if file}
                <span class="text-success">+{file.added}</span>
                <span class="text-destructive">−{file.deleted}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <RunSteps steps={item.steps} {ctx} />
  {/if}
</RowShell>
