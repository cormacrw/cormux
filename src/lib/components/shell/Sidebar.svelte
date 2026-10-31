<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { isDevBuild } from '$lib/build-mode'
  import { Button } from '$lib/components/ui/button'
  import * as ScrollArea from '$lib/components/ui/scroll-area'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { app, clickup, memory } from '$lib/state'
  import { cubicOut } from 'svelte/easing'
  import { flip } from 'svelte/animate'
  import { fade, fly } from 'svelte/transition'
  import Cpu from '@lucide/svelte/icons/cpu'
  import Kanban from '@lucide/svelte/icons/square-kanban'
  import Sparkle from '@lucide/svelte/icons/sparkle'
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
  import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus'
  import ThreadListRow from './ThreadListRow.svelte'
  import { motionMs } from '$lib/motion'
  import { sidebarWorkspaces } from '$lib/state/sidebar-workspaces.svelte'

  function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0) return
    void getCurrentWindow().startDragging()
  }

  // Full screen hides the traffic lights, so their row collapses and the sidebar moves up.
  let fullscreen = $state(false)

  // Keeps the rows' ages current.
  let nowMs = $state(Date.now())

  onMount(() => {
    const timer = setInterval(() => (nowMs = Date.now()), 60_000)
    return () => clearInterval(timer)
  })

  onMount(() => {
    const win = getCurrentWindow()
    const sync = () =>
      void win
        .isFullscreen()
        .then((value) => (fullscreen = Boolean(value)))
        .catch(() => {})
    sync()
    let unlisten: (() => void) | undefined
    let disposed = false
    // Entering or leaving full screen resizes the window.
    void win.onResized(sync).then((stop) => {
      if (disposed) stop()
      else unlisten = stop
    })
    return () => {
      disposed = true
      unlisten?.()
    }
  })
</script>

<nav
  class="sidebar-nav grid h-full min-h-0 grid-rows-[auto_auto_minmax(0,1fr)_auto] px-3.5 pb-3.5"
  aria-label="Harness"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={fullscreen ? 'titlebar h-3.5' : 'titlebar h-[52px]'}
    data-tauri-drag-region
    data-od-id="sidebar-titlebar"
    onmousedown={startWindowDrag}
  ></div>

  <div class="grid gap-0.5 pb-2">
    <button
      type="button"
      class="glass-well mb-2 flex h-[34px] w-full items-center gap-2 rounded-[10px] pr-1.5 pl-2.5 text-left text-sm text-muted-foreground transition-[background-color,color] duration-(--dur-fast) outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50"
      aria-label="Search or run a command (Command K)"
      data-od-id="sidebar-command-bar"
      onclick={() => app.requestCommandPalette()}
    >
      <Sparkle class="size-3.5 shrink-0 text-primary" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate">Search or start…</span>
      <kbd
        class="rounded-md bg-foreground/10 px-1.5 py-1 font-mono text-[10px] leading-none"
        aria-hidden="true">⌘K</kbd
      >
    </button>

    <Button
      variant="ghost"
      class="h-[30px] w-full justify-start gap-2 px-2 text-sm font-normal text-muted-foreground hover:text-foreground aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-foreground"
      aria-current={app.view === 'newThread' ? 'page' : undefined}
      data-od-id="nav-new-thread"
      onclick={() => app.openNewThread()}
    >
      <MessageSquarePlus
        class="size-4 shrink-0 opacity-80"
        aria-hidden="true"
      />
      <span class="flex-1 text-left">New thread</span>
      <kbd
        class="font-mono text-[10px] text-muted-foreground/80"
        aria-hidden="true">⌘N</kbd
      >
    </Button>

    {#if clickup.configured}
      <Button
        variant="ghost"
        class="h-[30px] w-full justify-start gap-2 px-2 text-sm font-normal text-muted-foreground hover:text-foreground aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-foreground"
        aria-current={app.view === 'sprint' ? 'page' : undefined}
        onclick={() => app.openSprint()}
      >
        <Kanban class="size-4 shrink-0 opacity-80" aria-hidden="true" />
        <span class="flex-1 text-left">Sprint</span>
        {#if clickup.progress.openTasks > 0}
          <span
            class="font-mono text-[10px] text-muted-foreground/80"
            title="Open tasks">{clickup.progress.openTasks}</span
          >
        {/if}
      </Button>
    {/if}
  </div>

  <ScrollArea.Root class="sidebar-scroll -mx-2 min-h-0 flex-1 px-2">
    <div class="flex flex-col">
      <div
        class="flex items-center justify-between px-2 pt-4 pb-1.5 text-[11px] font-semibold text-muted-foreground"
        id="side-threads-label"
      >
        <span>Threads</span>
        <span class="font-mono tracking-normal normal-case"
          >{sidebarWorkspaces.items.length}</span
        >
      </div>
      {#if sidebarWorkspaces.items.length === 0}
        <p class="px-2 py-1.5 text-xs text-muted-foreground">No threads yet</p>
      {:else}
        <!-- What needs you first; ⌘1–⌘9 follow this order (sidebar-workspaces.svelte.ts). -->
        <ul
          class="grid list-none gap-1 p-0"
          aria-labelledby="side-threads-label"
          data-od-id="thread-list"
        >
          {#each sidebarWorkspaces.items as item (item.id)}
            <li
              animate:flip={{ duration: motionMs(320), easing: cubicOut }}
              in:fly={{ y: -10, duration: motionMs(240), easing: cubicOut }}
              out:fade={{ duration: motionMs(140) }}
            >
              <ThreadListRow {item} {nowMs} />
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </ScrollArea.Root>

  <div class="flex items-center gap-2 px-2 pt-2">
    <div class="flex min-w-0 flex-1 items-center">
      <Tooltip.Root>
        <Tooltip.Trigger
          class="flex cursor-default items-center gap-2 rounded-sm text-xs text-muted-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          aria-label={`Memory used: ${memory.label}`}
        >
          <Cpu class="size-3.5 shrink-0" aria-hidden="true" />
          <span class="font-mono tabular-nums">{memory.label}</span>
        </Tooltip.Trigger>
        <Tooltip.Content side="top">Memory used</Tooltip.Content>
      </Tooltip.Root>
      {#if isDevBuild}
        <span
          class="ml-2 rounded-md bg-warning px-1.5 py-0.5 font-mono text-[10px] font-bold tracking-wider text-background"
          title="Development build, with its own data"
          data-od-id="dev-build-badge">DEV</span
        >
      {/if}
    </div>
    <Button
      variant="ghost"
      size="icon"
      class="size-8 shrink-0 aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-foreground"
      aria-label="Settings"
      title="Settings"
      aria-current={app.view === 'settings' ? 'page' : undefined}
      onclick={() => app.openSettings()}
    >
      <SlidersHorizontal class="size-4" aria-hidden="true" />
    </Button>
  </div>
</nav>
