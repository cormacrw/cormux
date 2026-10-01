<script lang="ts">
  import { Badge } from '$lib/components/ui/badge'
  import { app, repos, threads } from '$lib/state'
  import {
    SCRATCH_STATUS_LABEL,
    scratchStatus,
    type Scratch,
  } from '$lib/state/scratches.svelte'
  import { plural, type StatusDotVariant } from '$lib/sidebar/status'
  import StatusDot from './StatusDot.svelte'

  let { scratch }: { scratch: Scratch } = $props()

  const thread = $derived(threads.getById(scratch.threadId))
  const status = $derived(scratchStatus(thread))
  const statusWord = $derived(SCRATCH_STATUS_LABEL[status])
  const repoName = $derived(repos.getById(scratch.repoId)?.name ?? '')
  const pendingApprovals = $derived(thread?.pendingApprovals ?? 0)
  const dotVariant = $derived<StatusDotVariant>(
    status === 'working' ? 'running' : status === 'idle' ? 'idle' : 'paused',
  )
  const isCurrent = $derived(
    app.view === 'scratch' && app.scratchId === scratch.id,
  )

  const ariaLabel = $derived.by(() => {
    const parts = [scratch.title, statusWord]
    if (repoName) parts.push(`in ${repoName}`)
    if (pendingApprovals > 0) {
      parts.push(plural(pendingApprovals, 'approval', 'approvals') + ' waiting')
    }
    return parts.join(', ')
  })
</script>

<li>
  <button
    type="button"
    class="grid w-full grid-cols-[16px_minmax(0,1fr)_auto] items-center gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-sidebar-accent/80 aria-[current=page]:bg-sidebar-accent"
    aria-current={isCurrent ? 'page' : undefined}
    aria-label={ariaLabel}
    onclick={() => app.openScratch(scratch.id)}
  >
    <StatusDot variant={dotVariant} />
    <span class="min-w-0 grid" aria-hidden="true">
      <span class="truncate text-sm text-sidebar-foreground"
        >{scratch.title}</span
      >
      <span class="truncate text-xs text-muted-foreground">
        {statusWord}{#if repoName}
          · {repoName}{/if}
      </span>
    </span>
    {#if pendingApprovals > 0}
      <Badge
        variant="outline"
        class="min-w-[18px] justify-center border-warning/40 bg-warning/15 px-1.5 font-mono text-[10px] text-warning"
        aria-hidden="true"
      >
        {pendingApprovals}
      </Badge>
    {:else}
      <span aria-hidden="true"></span>
    {/if}
  </button>
</li>
