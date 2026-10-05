<script lang="ts">
  import { app, scratches, settings, shellDialogs, threads } from '$lib/state'
  import {
    SCRATCH_STATUS_LABEL,
    scratchStatus,
    type Scratch,
  } from '$lib/state/scratches.svelte'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { onMount, untrack } from 'svelte'
  import ScratchStatusBadge from './ScratchStatusBadge.svelte'

  let { scratch }: { scratch: Scratch } = $props()

  const status = $derived(scratchStatus(threads.getById(scratch.threadId)))
  // Read once: later re-renders must not replay the entrance.
  const entering = untrack(() => !scratches.seenIds.includes(scratch.id))

  onMount(() => {
    if (settings.reduceMotion) scratches.markSeen(scratch.id)
  })
</script>

<article
  class="sess-card group/sess flex min-h-[60px] min-w-0 items-stretch rounded-[999px_999px_999px_999px] bg-card text-card-foreground shadow-lift-1 transition-transform duration-200 ease-[var(--squish)] hover:-translate-y-0.5 {entering &&
  !settings.reduceMotion
    ? 'animate-in fade-in slide-in-from-bottom-1 duration-300'
    : ''}"
  data-od-id="session-card-{scratch.id}"
  onanimationend={() => scratches.markSeen(scratch.id)}
>
  <button
    type="button"
    class="sess-open flex min-w-0 flex-1 items-center gap-3 rounded-l-full py-2.5 pr-2 pl-6 text-left outline-none"
    data-action="open-scratch"
    data-arg={scratch.id}
    aria-label="{scratch.title}, {scratch.repoId}, {SCRATCH_STATUS_LABEL[
      status
    ]}"
    onclick={() => app.openScratch(scratch.id)}
  >
    <span class="flex min-w-0 flex-1 flex-col">
      <span class="truncate text-[16px] font-bold">{scratch.title}</span>
      <span class="truncate text-[13px] font-semibold text-muted-foreground"
        >{scratch.repoId}</span
      >
    </span>
    <ScratchStatusBadge {status} />
  </button>
  <button
    type="button"
    class="mr-3 flex size-9 shrink-0 items-center justify-center self-center rounded-full text-muted-foreground opacity-0 outline-none transition-opacity group-hover/sess:opacity-100 focus-visible:opacity-100 hover:bg-foreground/[0.07] hover:text-brick-ink"
    data-action="delete-scratch"
    data-arg={scratch.id}
    title="Delete scratch"
    aria-label="Delete {scratch.title}"
    onclick={() => shellDialogs.openEndScratch(scratch.id, true)}
  >
    <Trash2 class="size-4" aria-hidden="true" />
  </button>
</article>
