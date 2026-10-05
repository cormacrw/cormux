<script lang="ts">
  import { buddyFace } from '$lib/clay/identity'
  import Buddy from '$lib/components/clay/Buddy.svelte'
  import CountBead from '$lib/components/clay/CountBead.svelte'
  import { app, repos, threads } from '$lib/state'
  import {
    SCRATCH_STATUS_LABEL,
    scratchStatus,
    type Scratch,
  } from '$lib/state/scratches.svelte'
  import { plural, statusDotVariantForThread } from '$lib/sidebar/status'

  let { scratch }: { scratch: Scratch } = $props()

  const thread = $derived(threads.getById(scratch.threadId))
  const status = $derived(scratchStatus(thread))
  const statusWord = $derived(SCRATCH_STATUS_LABEL[status])
  const repoName = $derived(repos.getById(scratch.repoId)?.name ?? '')
  const pendingApprovals = $derived(thread?.pendingApprovals ?? 0)
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
    class="side-row grid w-full grid-cols-[30px_minmax(0,1fr)_auto] items-center gap-3 rounded-[20px_24px_18px_22px] px-3 py-2 text-left text-sidebar-foreground outline-none transition-[background-color,box-shadow] duration-150 hover:bg-white/10 focus-visible:shadow-[0_0_0_3px_var(--sidebar-ring)] aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground aria-[current=page]:shadow-[inset_0_-4px_0_rgb(0_0_0/0.1),0_5px_10px_rgb(0_0_0/0.18)]"
    aria-current={isCurrent ? 'page' : undefined}
    aria-label={ariaLabel}
    onclick={() => app.openScratch(scratch.id)}
  >
    <Buddy
      color="var(--clay-blush)"
      face={buddyFace(
        statusDotVariantForThread({
          status: thread?.status ?? 'idle',
          paused: thread?.paused ?? false,
          activity: thread?.activity ?? '',
        }),
        pendingApprovals > 0,
      )}
      breathe={thread?.status === 'running'}
      size={30}
    />
    <span class="min-w-0 grid" aria-hidden="true">
      <span class="truncate text-[16px] leading-tight font-bold"
        >{scratch.title}</span
      >
      <span class="truncate text-[13.5px] leading-tight opacity-80">
        {statusWord}{#if repoName}
          · {repoName}{/if}
      </span>
    </span>
    {#if pendingApprovals > 0}
      <CountBead count={pendingApprovals} />
    {:else}
      <span aria-hidden="true"></span>
    {/if}
  </button>
</li>
