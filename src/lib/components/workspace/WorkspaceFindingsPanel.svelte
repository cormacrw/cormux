<script lang="ts">
  import { findings } from '$lib/state'

  let {
    workspaceId,
    headingRef = $bindable(),
  }: {
    workspaceId: string
    headingRef?: HTMLHeadingElement | undefined
  } = $props()

  const rows = $derived(findings.forWorkspace(workspaceId))
  const openCount = $derived(
    rows.filter((row) => row.status === 'open').length,
  )
</script>

<div
  id="findings-panel"
  role="tabpanel"
  aria-labelledby="thread-tab-findings"
  data-od-id="findings"
  class="flex min-h-0 flex-1 flex-col gap-3"
>
  <header class="border-b border-border/60 pb-3">
    <h2
      bind:this={headingRef}
      id="fnd-heading"
      tabindex="-1"
      class="text-lg font-semibold tracking-tight outline-none"
    >
      Review findings
    </h2>
    <p class="text-sm text-muted-foreground">
      {openCount} open · full findings UI lands in COR-20
    </p>
  </header>
  {#if rows.length === 0}
    <p class="text-sm text-muted-foreground">No findings for this workspace.</p>
  {:else}
    <ul class="space-y-2 text-sm">
      {#each rows as row (row.id)}
        <li
          class="rounded-md border border-border/70 px-3 py-2"
          data-od-id="finding-{row.id}"
        >
          <p class="font-medium">{row.title}</p>
          <p class="text-xs text-muted-foreground">{row.severity} · {row.status}</p>
        </li>
      {/each}
    </ul>
  {/if}
</div>
