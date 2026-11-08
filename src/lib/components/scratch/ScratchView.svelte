<script lang="ts">
  import { windowDrag } from '$lib/primitives/window-drag'
  import { tick } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import ShortcutKbd from '$lib/primitives/ShortcutKbd.svelte'
  import { shortcutAria } from '$lib/keyboard/shortcuts'
  import ThreadConversation from '$lib/components/workspace/thread/ThreadConversation.svelte'
  import { prs, repos, scratches, shellDialogs } from '$lib/state'
  import type { ConversationSource } from '$lib/components/workspace/thread/rows/context'
  import { provideMediaHost } from '$lib/components/workspace/thread/reply/media-host'
  import { nav } from '$lib/core/nav.svelte'
  import type { Scratch } from '$lib/state/scratches.svelte'
  import { engineDisplayName } from '$lib/engine/engines'

  const scratch = $derived<Scratch | undefined>(
    nav.scratchId ? scratches.getById(nav.scratchId) : undefined,
  )
  const repo = $derived(scratch ? repos.getById(scratch.repoId) : undefined)
  const repoPath = $derived(repo?.path ?? scratch?.repoId ?? '')
  const engineName = $derived(engineDisplayName(scratch?.engine ?? 'claude'))

  let headingEl = $state<HTMLHeadingElement | null>(null)
  let sectionEl: HTMLElement | undefined = $state()
  provideMediaHost(() => sectionEl)

  // No worktree and no Changes to open: file references stay plain text.
  const source: ConversationSource = {
    workdir: undefined,
    diffFile: () => undefined,
    findPr: (repoFullName, num) => prs.find(repoFullName, num),
    revealFile: undefined,
    workspaceId: undefined,
    openFindings: undefined,
  }

  $effect(() =>
    nav.onArrival(() => {
      if (nav.view === 'scratch') void tick().then(() => headingEl?.focus())
    }),
  )

  function onKeydown(event: KeyboardEvent) {
    if (!scratch || !shellDialogs.allows(event, 'deleteScratch')) return
    event.preventDefault()
    shellDialogs.open('endScratch', { scratchId: scratch.id })
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if scratch}
  <section
    bind:this={sectionEl}
    class="view relative flex min-h-0 flex-1 flex-col"
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
          <ShortcutKbd id="deleteScratch" />
        </Button>
      </div>
    </header>

    <ThreadConversation
      threadId={scratch.threadId}
      role=""
      engine={engineName}
      {source}
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
