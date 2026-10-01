<script lang="ts">
  import { Badge } from '$lib/components/ui/badge'
  import { app, workspaceRecords } from '$lib/state'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    plural,
    statusDotVariantForWorkspace,
    workspaceStatusWord,
  } from '$lib/sidebar/status'
  import StatusDot from './StatusDot.svelte'

  let { workspace, agentCount }: { workspace: Workspace; agentCount: number } =
    $props()

  const statusInput = $derived({
    lifecycle: workspace.lifecycle,
    paused: workspace.paused,
    activityText: workspace.activityText,
  })

  const statusWord = $derived(workspaceStatusWord(statusInput))
  const dotVariant = $derived(statusDotVariantForWorkspace(statusInput))
  const isCurrent = $derived(
    app.view === 'workspace' && app.workspaceId === workspace.id,
  )

  // The workspace's dev app, shown so a running server is findable without opening it.
  const appRuntime = $derived(workspaceRecords.runtime(workspace.id))
  const appRunning = $derived(appRuntime.appStatus === 'running')

  const ariaLabel = $derived.by(() => {
    const parts = [workspace.name, statusWord, plural(agentCount, 'agent')]
    if (appRunning) {
      parts.push(
        appRuntime.port
          ? `app running on port ${appRuntime.port}`
          : 'app running',
      )
    }
    if (workspace.pendingApprovals > 0) {
      parts.push(
        plural(workspace.pendingApprovals, 'approval', 'approvals') +
          ' waiting',
      )
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
    onclick={() => app.openWorkspace(workspace.id)}
  >
    <StatusDot variant={dotVariant} />
    <span class="min-w-0 grid" aria-hidden="true">
      <span class="truncate text-sm text-sidebar-foreground"
        >{workspace.name}</span
      >
      <span class="truncate text-xs text-muted-foreground">
        {statusWord} · {plural(agentCount, 'agent')}
      </span>
    </span>
    {#if workspace.pendingApprovals > 0}
      <Badge
        variant="outline"
        class="min-w-[18px] justify-center border-warning/40 bg-warning/15 px-1.5 font-mono text-[10px] text-warning"
        aria-hidden="true"
      >
        {workspace.pendingApprovals}
      </Badge>
    {:else if appRunning}
      <span
        class="flex items-center gap-1 font-mono text-[10px] text-emerald-600"
        title={appRuntime.port
          ? `App running on localhost:${appRuntime.port}`
          : 'App running'}
        aria-hidden="true"
      >
        <span class="size-1.5 rounded-full bg-emerald-500"></span>
        {#if appRuntime.port}:{appRuntime.port}{/if}
      </span>
    {:else}
      <span aria-hidden="true"></span>
    {/if}
  </button>
</li>
