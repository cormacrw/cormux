<script lang="ts">
  import { buddyFace } from '$lib/clay/identity'
  import Buddy from '$lib/components/clay/Buddy.svelte'
  import CountBead from '$lib/components/clay/CountBead.svelte'
  import { app, workspaces } from '$lib/state'
  import type { Thread } from '$lib/state/threads.svelte'
  import { engineDisplayName } from '$lib/sidebar/engine'
  import {
    plural,
    statusDotVariantForThread,
    threadActivityLine,
  } from '$lib/sidebar/status'

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
  // Agents wear their status: coral while working, pond when waiting on you, pebble when idle.
  const color = $derived(
    thread.pendingApprovals > 0
      ? 'var(--clay-pond)'
      : dotVariant === 'running' || dotVariant === 'provisioning'
        ? 'var(--clay-coral)'
        : dotVariant === 'paused'
          ? 'var(--clay-marigold)'
          : 'var(--clay-pebble)',
  )

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
    class="side-row grid w-full grid-cols-[30px_minmax(0,1fr)_auto] items-center gap-3 rounded-[20px_24px_18px_22px] px-3 py-2 text-left text-sidebar-foreground outline-none transition-[background-color,box-shadow] duration-150 hover:bg-white/10 focus-visible:shadow-[0_0_0_3px_var(--sidebar-ring)] aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground aria-[current=page]:shadow-[inset_0_-4px_0_rgb(0_0_0/0.1),0_5px_10px_rgb(0_0_0/0.18)]"
    aria-label={ariaLabel}
    onclick={() => app.openWorkspace(thread.workspaceId, thread.id)}
  >
    <Buddy
      {color}
      face={buddyFace(dotVariant, thread.pendingApprovals > 0)}
      breathe={dotVariant === 'running'}
      size={30}
    />
    <span class="min-w-0 grid" aria-hidden="true">
      <span class="truncate text-[16px] leading-tight font-bold">
        {thread.role} in {workspaceName}
      </span>
      <span class="truncate text-[13.5px] leading-tight opacity-80"
        >{activity}</span
      >
    </span>
    {#if thread.pendingApprovals > 0}
      <CountBead count={thread.pendingApprovals} />
    {:else}
      <span aria-hidden="true"></span>
    {/if}
  </button>
</li>
