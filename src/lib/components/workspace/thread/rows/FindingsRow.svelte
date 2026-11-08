<script lang="ts">
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import List from '@lucide/svelte/icons/list'
  import { severitySummary } from '$lib/findings/format'
  import { nav } from '$lib/core/nav.svelte'
  import { findings } from '$lib/findings/store.svelte'
  import { openWorkspaceFindings } from '$lib/nav/open'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import type { RowContext } from './context'
  import RowShell from './RowShell.svelte'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'findings' }>; ctx: RowContext } =
    $props()

  const rows = $derived(findings.forWorkspace(item.workspaceId))

  function openFindings() {
    openWorkspaceFindings(item.workspaceId, nav.threadId ?? undefined)
  }
</script>

<RowShell id={item.id} {ctx} pop>
  <div
    class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
    data-od-id="findings-card"
  >
    <List class="size-3 shrink-0" aria-hidden="true" />
    <span class="shrink-0">Review findings</span>
    <span class="truncate text-muted-foreground/80"
      >· {rows.length ? severitySummary(rows) : 'No findings'}</span
    >
    <button
      type="button"
      class="inline-flex shrink-0 items-center gap-0.5 font-medium text-foreground/80 underline-offset-2 hover:text-foreground hover:underline"
      data-od-id="findings-card-open"
      onclick={openFindings}
    >
      Open findings
      <ArrowRight class="size-3" aria-hidden="true" />
    </button>
  </div>
</RowShell>
