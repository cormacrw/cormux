<script lang="ts">
  import { switchConversation } from '$lib/thread/actions'
  import { plural, pluralWord } from '$lib/format/plural'
  import { onDestroy } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { entities } from '$lib/state'
  import { threadTimeline } from '$lib/thread/timeline-store.svelte'
  import { settings } from '$lib/settings/store.svelte'
  import { openWorkspace } from '$lib/nav/open'
  import { wordmark } from '$lib/background/wordmark.svelte'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { engineDisplayName } from '$lib/engine/engines'
  import Pause from '@lucide/svelte/icons/pause'
  import Bot from '@lucide/svelte/icons/bot'
  import AgentSpinner from './AgentSpinner.svelte'
  import ThreadComposer from './ThreadComposer.svelte'
  import ThreadConversation from './ThreadConversation.svelte'
  import ThreadHero from './ThreadHero.svelte'
  import ThreadIntro from './ThreadIntro.svelte'
  import ThreadReferences from './ThreadReferences.svelte'

  let {
    workspace,
    thread,
  }: {
    workspace: Workspace
    thread: Thread
  } = $props()

  const threadId = $derived(thread.id)
  const view = $derived(threadTimeline.threadView(threadId))
  const live = $derived(view.live)
  // An empty conversation shows a greeting instead of the last one's messages.
  const showHero = $derived(view.loaded && view.items.length === 0)
  let conversation: ThreadConversation | undefined = $state()

  // Experimental: empty unless References is on in Settings.
  const threadEntities = $derived(
    settings.experimentalReferences ? entities.forThread(thread.id) : [],
  )

  const AGENTS_SHOWN = 4
  // The live status floats over the end of the timeline, which leaves room for all of it.
  let liveStatusHeight = $state(0)

  $effect(() => {
    if (view.loaded) wordmark.showConversation(!showHero)
  })

  // The greeting offers earlier conversations to resume, however far back they are.
  $effect(() => {
    if (showHero && view.hasOlder) void threadTimeline.loadOlder(threadId)
  })

  onDestroy(() => wordmark.reset())
</script>

<div
  id="thread-panel"
  data-od-id="thread-panel"
  class="relative flex min-h-0 flex-1 flex-col"
  data-references={threadEntities.length > 0}
>
  <ThreadConversation
    bind:this={conversation}
    {threadId}
    role={thread.role}
    engine={engineDisplayName(thread.engine)}
    workspaceId={workspace.id}
    contentClass={[
      'pt-3',
      // The greeting sits at the bottom, its box reaching down to the prompt
      // bar so it can fall away behind it (see ThreadHero).
      showHero
        ? 'flex min-h-full flex-col overflow-clip'
        : live
          ? 'pb-14'
          : 'pb-4',
    ]}
    padBottom={live && !showHero && liveStatusHeight
      ? `calc(${liveStatusHeight}px + 1.75rem)`
      : undefined}
  >
    {#snippet aside()}
      {#if threadEntities.length}
        <!-- Docks in the gutter beside the conversation, above the prompt bar. -->
        <div class="references-dock">
          <ThreadReferences entities={threadEntities} />
        </div>
      {/if}
    {/snippet}
    {#snippet hero(nowMs: number)}
      <ThreadHero
        greeting="{threadId}:{view.conversations.current.id}"
        who={{
          role: thread.role,
          branch: workspace.branch,
          workspace: workspace.name,
        }}
        earlier={view.conversations.earlier}
        {nowMs}
        onResume={(to) => switchConversation(threadId, to)}
      />
    {/snippet}
    {#snippet intro(nowMs: number)}
      <ThreadIntro {thread} {workspace} {nowMs} />
    {/snippet}
  </ThreadConversation>

  {#snippet liveStatus()}
    {@const agents = live?.agents ?? []}
    <div
      class="flex max-w-full min-w-0 flex-col items-start gap-1"
      bind:clientHeight={liveStatusHeight}
    >
      {#if agents.length}
        <ul
          class="live-status glass-pane max-w-full min-w-0 space-y-1 rounded-[14px] py-1.5 pr-3.5 pl-3 text-xs shadow-none"
          data-od-id="running-agents"
          aria-label="Running agents"
        >
          {#each agents.slice(0, AGENTS_SHOWN) as agent (agent.id)}
            <li class="flex min-w-0 items-center gap-1.5">
              <Bot
                class="size-3 shrink-0 animate-pulse text-primary"
                aria-hidden="true"
              />
              <span class="shrink-0 font-medium text-foreground/85"
                >{agent.description}</span
              >
              {#if agent.latest}
                <span
                  class="truncate text-muted-foreground"
                  title={agent.latest}>· {agent.latest}</span
                >
              {/if}
              {#if agent.steps}
                <span class="shrink-0 text-muted-foreground/70"
                  >· {plural(agent.steps, 'step')}</span
                >
              {/if}
            </li>
          {/each}
          {#if agents.length > AGENTS_SHOWN}
            <li class="pl-[1.125rem] text-muted-foreground">
              +{agents.length - AGENTS_SHOWN} more
            </li>
          {/if}
        </ul>
      {/if}
      <div
        class="live-status glass-pane flex max-w-full min-w-0 items-center gap-2 rounded-full py-1.5 pr-3.5 pl-3 text-xs shadow-none"
        data-od-id="live-status"
      >
        {#if live?.paused}
          <Pause
            class="size-3 shrink-0 text-muted-foreground"
            aria-hidden="true"
          />
        {:else}
          <AgentSpinner class="mr-1" />
        {/if}
        <span
          class={[
            'shrink-0 font-medium text-foreground/85',
            // Re-added on resume as the spinner remounts, so both restart the beat together.
            !live?.paused && 'agent-working-text',
          ]}>{live?.title}</span
        >
        {#if live?.subtitle && live.subtitle !== live.title}
          <span class="truncate text-muted-foreground">· {live.subtitle}</span>
        {/if}
        {#if agents.length}
          <span class="shrink-0 text-muted-foreground"
            >· {agents.length}
            {pluralWord(agents.length, 'agent')} running</span
          >
        {/if}
      </div>
    </div>
  {/snippet}

  {#if thread.retired}
    <div class="px-1 pt-2 pb-4">
      <p
        class="glass-pane mx-auto flex max-w-[760px] items-center justify-between gap-3 rounded-[18px] px-4 py-3 text-sm text-muted-foreground"
        data-od-id="thread-read-only"
      >
        This earlier thread is read-only.
        <Button
          variant="secondary"
          size="sm"
          onclick={() => openWorkspace(workspace.id)}
        >
          Go to the current thread
        </Button>
      </p>
    </div>
  {:else}
    <ThreadComposer
      {thread}
      newSession
      status={live ? liveStatus : undefined}
      onSent={() => conversation?.followEnd()}
    />
  {/if}
</div>

<style>
  #thread-panel {
    container: thread / inline-size;
  }

  /* Hidden until the pane is wide enough to hold it beside a readable conversation. */
  .references-dock {
    display: none;
    position: absolute;
    top: 0.75rem;
    right: 1.25rem;
    bottom: 0.75rem;
    z-index: 5;
    width: 248px;
    flex-direction: column;
    pointer-events: none;
  }

  .references-dock > :global(*) {
    pointer-events: auto;
  }

  @container thread (min-width: 900px) {
    .references-dock {
      display: flex;
    }
  }

  /* Short of a gutter that fits it, the conversation and prompt bar move over to make room. */
  @container thread (min-width: 900px) and (max-width: 1319px) {
    [data-references='true'] :global(#thread-scroll),
    [data-references='true'] :global(.composer-wrap) {
      padding-right: 280px;
    }
  }

  /* Floats in a beat after a send, once a first prompt's greeting has fallen away. */
  .live-status {
    animation: live-status-in var(--dur-med) var(--ease-out-snappy) 400ms
      backwards;
  }

  @keyframes live-status-in {
    from {
      opacity: 0;
      transform: translate3d(0, 6px, 0);
    }
  }
</style>
