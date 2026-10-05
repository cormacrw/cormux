<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { isDevBuild } from '$lib/build-mode'
  import * as ScrollArea from '$lib/components/ui/scroll-area'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import {
    app,
    clickup,
    memory,
    repos,
    scratches,
    threads,
    todos,
    workspaces,
  } from '$lib/state'
  import { isActiveThread } from '$lib/sidebar/status'
  import Search from '@lucide/svelte/icons/search'
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
  import SidebarAgentRow from './SidebarAgentRow.svelte'
  import SidebarRepoRow from './SidebarRepoRow.svelte'
  import SidebarScratchRow from './SidebarScratchRow.svelte'
  import SidebarWorkspaceRow from './SidebarWorkspaceRow.svelte'

  const workingAgents = $derived(
    threads.sidebarAgents.filter(
      (thread) =>
        isActiveThread({
          status: thread.status,
          paused: thread.paused,
          activity: thread.activity,
        }) && !thread.paused,
    ).length,
  )

  function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0) return
    void getCurrentWindow().startDragging()
  }
</script>

{#snippet navItem(
  label: string,
  bead: string,
  current: boolean,
  onclick: () => void,
  trailing: string,
  trailingTitle?: string,
)}
  <button
    type="button"
    class="side-item group flex h-11 w-full items-center gap-3 rounded-[22px_26px_20px_24px] px-4 text-left font-display text-[17px] font-extrabold text-sidebar-foreground outline-none transition-[background-color,box-shadow,transform] duration-150 hover:bg-white/10 focus-visible:shadow-[0_0_0_3px_var(--sidebar-ring)] aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground aria-[current=page]:shadow-[inset_0_-4px_0_rgb(0_0_0/0.1),0_5px_10px_rgb(0_0_0/0.18)]"
    aria-current={current ? 'page' : undefined}
    {onclick}
  >
    <span class="bead size-3.5" style="background: {bead}"></span>
    <span class="min-w-0 flex-1 truncate">{label}</span>
    {#if trailing}
      <span
        class="font-mono text-[11.5px] font-semibold opacity-80"
        title={trailingTitle}
        aria-hidden={trailingTitle ? undefined : 'true'}>{trailing}</span
      >
    {/if}
  </button>
{/snippet}

{#snippet sectionHead(label: string, id: string, trailing: string)}
  <div
    class="flex items-center justify-between px-3 pt-6 pb-2 text-sidebar-head"
    {id}
  >
    <span class="eyebrow">{label}</span>
    <span class="font-display text-[14px] font-extrabold">{trailing}</span>
  </div>
{/snippet}

<nav
  class="sidebar-nav relative grid h-full min-h-0 grid-rows-[auto_auto_minmax(0,1fr)_auto] pr-4 pl-3 pb-3 text-sidebar-foreground"
  aria-label="Harness"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="titlebar flex h-10 items-center justify-end"
    data-tauri-drag-region
    onmousedown={startWindowDrag}
  >
    {#if isDevBuild}
      <span
        class="pointer-events-none rounded-full bg-marigold px-2 py-0.5 font-display text-[11px] font-extrabold tracking-wider text-cocoa shadow-[inset_0_-2px_0_rgb(0_0_0/0.15)]"
        title="Development build, with its own data"
        data-od-id="dev-build-badge">DEV</span
      >
    {/if}
  </div>

  <div class="grid gap-1 pb-1">
    <button
      type="button"
      class="squish mb-3 flex h-12 w-full items-center gap-3 rounded-[24px_28px_22px_26px] bg-sidebar-accent px-4 text-left text-[17px] font-semibold text-sidebar-accent-foreground outline-none clay focus-visible:shadow-[var(--clay-shade),0_0_0_3px_var(--sidebar-ring)]"
      aria-label="Search or run a command (Command K)"
      onclick={() => app.requestCommandPalette()}
    >
      <Search class="size-[18px] shrink-0" strokeWidth={2.5} aria-hidden="true" />
      <span class="min-w-0 flex-1 opacity-75">Search</span>
      <kbd class="font-mono text-[11.5px] font-semibold" aria-hidden="true"
        >⌘K</kbd
      >
    </button>

    {@render navItem(
      'Homebase',
      'var(--clay-plum)',
      app.view === 'homebase',
      () => app.openHomebase(),
      '⌘H',
    )}
    {@render navItem(
      'TODOs',
      'var(--clay-leaf)',
      app.view === 'todos',
      () => app.openTodos(),
      todos.items.length > 0 ? String(todos.items.length) : '',
    )}
    {#if clickup.configured}
      {@render navItem(
        'Sprint',
        'var(--clay-pond)',
        app.view === 'sprint',
        () => app.openSprint(),
        clickup.progress.openTasks > 0
          ? String(clickup.progress.openTasks)
          : '',
        'Open tasks',
      )}
    {/if}
    {@render navItem(
      'New workspace',
      'var(--clay-coral)',
      false,
      () => app.requestNewWorkspace(),
      '⌘N',
    )}
  </div>

  <ScrollArea.Root class="sidebar-scroll -mr-3 min-h-0 flex-1 pr-3">
    <div class="flex flex-col pb-2">
      {@render sectionHead('Repos', 'side-repo-label', String(repos.items.length))}
      <ul class="grid list-none gap-0.5 p-0" aria-labelledby="side-repo-label">
        {#if repos.items.length === 0}
          <li class="px-3 py-1.5 text-sm text-sidebar-foreground/70">
            No repos yet
          </li>
        {:else}
          {#each repos.items as repo (repo.id)}
            <SidebarRepoRow {repo} />
          {/each}
        {/if}
      </ul>

      {@render sectionHead(
        'Workspaces',
        'side-ws-label',
        String(workspaces.liveItems.length),
      )}
      <ul class="grid list-none gap-0.5 p-0" aria-labelledby="side-ws-label">
        {#if workspaces.sidebarItems.length === 0}
          <li class="px-3 py-1.5 text-sm text-sidebar-foreground/70">
            No workspaces yet
          </li>
        {:else}
          {#each workspaces.sidebarItems as workspace (workspace.id)}
            <SidebarWorkspaceRow
              {workspace}
              agentCount={threads.forWorkspace(workspace.id).length}
            />
          {/each}
        {/if}
      </ul>

      {#if scratches.items.length > 0}
        {@render sectionHead(
          'Scratches',
          'side-scratch-label',
          String(scratches.items.length),
        )}
        <ul
          class="grid list-none gap-0.5 p-0"
          aria-labelledby="side-scratch-label"
        >
          {#each scratches.items as scratch (scratch.id)}
            <SidebarScratchRow {scratch} />
          {/each}
        </ul>
      {/if}

      {@render sectionHead('Agents', 'side-ag-label', `${workingAgents} working`)}
      <ul class="grid list-none gap-0.5 p-0" aria-labelledby="side-ag-label">
        {#if threads.sidebarAgents.length === 0}
          <li class="px-3 py-1.5 text-sm text-sidebar-foreground/70">
            No agents running
          </li>
        {:else}
          {#each threads.sidebarAgents as thread (thread.id)}
            <SidebarAgentRow {thread} />
          {/each}
        {/if}
      </ul>
    </div>
  </ScrollArea.Root>

  <div
    class="flex items-center gap-2 border-t border-sidebar-border pt-3 pl-1"
  >
    <div class="flex min-w-0 flex-1 items-center">
      <Tooltip.Root>
        <Tooltip.Trigger
          class="flex cursor-default items-center gap-2.5 rounded-full text-sidebar-foreground outline-none focus-visible:shadow-[0_0_0_3px_var(--sidebar-ring)]"
          aria-label={`Memory used: ${memory.label}`}
        >
          <span class="bead size-3.5 bg-leaf" aria-hidden="true"></span>
          <span class="font-display text-[15px] font-extrabold">Memory</span>
          <span class="font-mono text-[13px] font-semibold tabular-nums"
            >{memory.label}</span
          >
        </Tooltip.Trigger>
        <Tooltip.Content side="top">Memory used</Tooltip.Content>
      </Tooltip.Root>
    </div>
    <button
      type="button"
      class="squish grid size-11 shrink-0 place-items-center rounded-full bg-custard text-cocoa outline-none clay focus-visible:shadow-[var(--clay-shade),0_0_0_3px_var(--sidebar-ring)] aria-[current=page]:bg-primary"
      aria-label="Settings"
      title="Settings"
      aria-current={app.view === 'settings' ? 'page' : undefined}
      onclick={() => app.openSettings()}
    >
      <SlidersHorizontal class="size-[18px]" strokeWidth={2.5} aria-hidden="true" />
    </button>
  </div>
</nav>
