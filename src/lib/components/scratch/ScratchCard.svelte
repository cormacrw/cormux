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
  class="sess-card group/sess flex min-h-[52px] min-w-0 items-stretch rounded-xl border border-border bg-card text-card-foreground transition-colors [@media(hover:hover)_and_(pointer:fine)]:hover:border-foreground/25 [@media(hover:hover)_and_(pointer:fine)]:hover:bg-muted/40 has-[.sess-open:active]:bg-muted/70 {entering &&
  !settings.reduceMotion
    ? 'animate-in fade-in slide-in-from-bottom-1 duration-300'
    : ''}"
  data-od-id="session-card-{scratch.id}"
  onanimationend={() => scratches.markSeen(scratch.id)}
>
  <button
    type="button"
    class="sess-open flex min-w-0 flex-1 items-center gap-3 rounded-l-xl py-2 pl-4 pr-2 text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset"
    data-action="open-scratch"
    data-arg={scratch.id}
    aria-label="{scratch.title}, {scratch.repoId}, {SCRATCH_STATUS_LABEL[
      status
    ]}"
    onclick={() => app.openScratch(scratch.id)}
  >
    <span class="flex min-w-0 flex-1 flex-col">
      <span class="truncate text-sm font-medium">{scratch.title}</span>
      <span class="truncate font-mono text-xs text-muted-foreground"
        >{scratch.repoId}</span
      >
    </span>
    <ScratchStatusBadge {status} />
  </button>
  <button
    type="button"
    class="flex size-11 shrink-0 items-center justify-center self-center rounded-lg text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset mr-1"
    data-action="delete-scratch"
    data-arg={scratch.id}
    title="Delete scratch"
    aria-label="Delete {scratch.title}"
    onclick={() => shellDialogs.openEndScratch(scratch.id, true)}
  >
    <Trash2 class="size-4" aria-hidden="true" />
  </button>
</article>
