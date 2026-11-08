<script lang="ts">
  import { windowDrag } from '$lib/primitives/window-drag'
  import { tick } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { Kbd } from '$lib/components/ui/kbd'
  import CommandIcon from '@lucide/svelte/icons/command'
  import { isHeaderShortcut } from '$lib/keyboard/header-shortcuts'
  import { shortcutAria } from '$lib/keyboard/shortcuts'
  import ThreadConversation from '$lib/components/workspace/thread/ThreadConversation.svelte'
  import { app, repos, scratches, shellDialogs, threads } from '$lib/state'
  import type { Scratch } from '$lib/state/scratches.svelte'
  import { engineDisplayName } from '$lib/engine/engines'

  const scratch = $derived<Scratch | undefined>(
    app.scratchId ? scratches.getById(app.scratchId) : undefined,
  )
  const thread = $derived(
    scratch ? threads.getById(scratch.threadId) : undefined,
  )
  const repo = $derived(scratch ? repos.getById(scratch.repoId) : undefined)
  // A removed repo falls back to its id.
  const repoPath = $derived(repo?.path ?? scratch?.repoId ?? '')
  const engineName = $derived(engineDisplayName(scratch?.engine ?? 'claude'))

  let headingEl = $state<HTMLHeadingElement | null>(null)

  $effect(() => {
    if (app.focusTarget !== 'scratch') return
    void app.focusGeneration
    void tick().then(() => headingEl?.focus())
  })

  function onKeydown(event: KeyboardEvent) {
    if (!scratch || !isHeaderShortcut(event, 'deleteScratch')) return
    event.preventDefault()
    shellDialogs.open('endScratch', { scratchId: scratch.id })
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if scratch && thread}
  <section
    class="view flex min-h-0 flex-1 flex-col"
    id="view-session"
    aria-label="Scratch"
    data-od-id="session-view"
  >
    <header
      class="ws-top flex items-center justify-between gap-4 border-b border-border/60 px-5 py-3"
      data-tauri-drag-region
      data-od-id="session-header"
      use:windowDrag
    >
      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        <h1
          bind:this={headingEl}
          id="sess-heading"
          tabindex="-1"
          class="text-[15px] font-medium tracking-tight outline-none [text-wrap:pretty]"
        >
          {scratch.title}
        </h1>
        <p
          class="truncate font-mono text-xs text-muted-foreground"
          title={repoPath}
        >
          {repoPath}
        </p>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <Button
          variant="secondary"
          size="xl"
          data-action="end-scratch"
          data-od-id="session-end"
          aria-keyshortcuts={shortcutAria('deleteScratch')}
          onclick={() =>
            shellDialogs.open('endScratch', { scratchId: scratch.id })}
        >
          Delete
          <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />E</Kbd>
        </Button>
      </div>
    </header>

    <ThreadConversation
      threadId={thread.id}
      role=""
      engine={engineName}
      workspaceId={null}
      scratch
      liveInline
      odId="session-thread"
      contentClass="pb-4 pt-6"
    />

    <p
      class="px-5 py-3 text-center text-xs text-muted-foreground"
      data-od-id="session-read-only"
    >
      This scratch is read-only. Start a new thread to carry on.
    </p>
  </section>
{/if}
