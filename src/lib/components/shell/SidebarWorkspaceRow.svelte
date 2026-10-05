<script lang="ts">
  import { clayColor } from '$lib/clay/identity'
  import Buddy from '$lib/components/clay/Buddy.svelte'
  import CountBead from '$lib/components/clay/CountBead.svelte'
  import { app, workspaceRecords } from '$lib/state'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    plural,
    workspaceStatusWord,
  } from '$lib/sidebar/status'

  let { workspace, agentCount }: { workspace: Workspace; agentCount: number } =
    $props()

  const statusInput = $derived({
    lifecycle: workspace.lifecycle,
    paused: workspace.paused,
    activityText: workspace.activityText,
  })

  const statusWord = $derived(workspaceStatusWord(statusInput))
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
    class="side-row grid w-full grid-cols-[30px_minmax(0,1fr)_auto] items-center gap-3 rounded-[20px_24px_18px_22px] px-3 py-2 text-left text-sidebar-foreground outline-none transition-[background-color,box-shadow] duration-150 hover:bg-white/10 focus-visible:shadow-[0_0_0_3px_var(--sidebar-ring)] aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground aria-[current=page]:shadow-[inset_0_-4px_0_rgb(0_0_0/0.1),0_5px_10px_rgb(0_0_0/0.18)]"
    aria-current={isCurrent ? 'page' : undefined}
    aria-label={ariaLabel}
    onclick={() => app.openWorkspace(workspace.id)}
  >
    <Buddy color={clayColor(workspace.id)} face="none" size={30} />
    <span class="min-w-0 grid" aria-hidden="true">
      <span class="truncate text-[16px] leading-tight font-bold"
        >{workspace.name}</span
      >
      <span class="truncate text-[13.5px] leading-tight opacity-80">
        {statusWord} · {plural(agentCount, 'agent')}
      </span>
    </span>
    {#if workspace.pendingApprovals > 0}
      <CountBead count={workspace.pendingApprovals} />
    {:else if appRunning}
      <span
        class="flex items-center gap-1 font-mono text-[11px] font-semibold"
        title={appRuntime.port
          ? `App running on localhost:${appRuntime.port}`
          : 'App running'}
        aria-hidden="true"
      >
        <span class="bead clay-glow size-2.5 bg-leaf"></span>
        {#if appRuntime.port}:{appRuntime.port}{/if}
      </span>
    {:else}
      <span aria-hidden="true"></span>
    {/if}
  </button>
</li>
