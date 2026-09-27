<script lang="ts">
  import { Badge } from '$lib/components/ui/badge'
  import { app, workspaces } from '$lib/state'
  import type { Thread } from '$lib/state/threads.svelte'
  import { engineDisplayName, engineMark } from '$lib/sidebar/engine'
  import {
    plural,
    statusDotVariantForThread,
    threadActivityLine,
  } from '$lib/sidebar/status'
  import StatusDot from './StatusDot.svelte'

  let { thread }: { thread: Thread } = $props()

  const workspaceName = $derived(
    workspaces.getById(thread.workspaceId)?.name ?? thread.workspaceId,
  )

  const statusInput = $derived({
    status: thread.status,
    paused: thread.paused,
    activity: thread.activity,
  })

  const activity = $derived(threadActivityLine(statusInput))
  const dotVariant = $derived(statusDotVariantForThread(statusInput))
  const mark = $derived(engineMark(thread.engine))

  const ariaLabel = $derived.by(() => {
    const parts = [
      thread.role,
      engineDisplayName(thread.engine),
      `in ${workspaceName}: ${activity}`,
    ]
    if (thread.pendingApprovals > 0) {
      parts.push(
        plural(thread.pendingApprovals, 'approval', 'approvals') + ' waiting',
      )
    }
    return parts.join(', ')
  })
</script>

<li>
  <button
    type="button"
    class="grid w-full grid-cols-[16px_minmax(0,1fr)_auto] items-center gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-sidebar-accent/80"
    aria-label={ariaLabel}
    onclick={() => app.openWorkspace(thread.workspaceId, thread.id)}
  >
    <span
      class="flex size-4 items-center justify-center rounded bg-sidebar-accent font-mono text-[8px] font-semibold text-muted-foreground"
      aria-hidden="true">{mark}</span
    >
    <span class="min-w-0 grid" aria-hidden="true">
      <span class="truncate text-sm text-sidebar-foreground">
        {thread.role}
        <span class="text-muted-foreground"> in {workspaceName}</span>
      </span>
      <span class="truncate text-xs text-muted-foreground">{activity}</span>
    </span>
    {#if thread.pendingApprovals > 0}
      <Badge
        variant="outline"
        class="min-w-[18px] justify-center border-warning/40 bg-warning/15 px-1.5 font-mono text-[10px] text-warning"
        aria-hidden="true"
      >
        {thread.pendingApprovals}
      </Badge>
    {:else}
      <StatusDot variant={dotVariant} class="justify-self-center" />
    {/if}
  </button>
</li>
