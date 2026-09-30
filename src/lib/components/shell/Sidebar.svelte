<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { isDevBuild } from '$lib/build-mode'
  import { Button } from '$lib/components/ui/button'
  import * as ScrollArea from '$lib/components/ui/scroll-area'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { app, memory, repos, threads, todos, workspaces } from '$lib/state'
  import { isActiveThread } from '$lib/sidebar/status'
  import Cpu from '@lucide/svelte/icons/cpu'
  import Layers from '@lucide/svelte/icons/layers'
  import ListTodo from '@lucide/svelte/icons/list-todo'
  import Plus from '@lucide/svelte/icons/plus'
  import Search from '@lucide/svelte/icons/search'
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
  import SidebarAgentRow from './SidebarAgentRow.svelte'
  import SidebarRepoRow from './SidebarRepoRow.svelte'
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

<nav
  class="sidebar-nav grid h-full min-h-0 grid-rows-[auto_auto_minmax(0,1fr)_auto] px-2 pb-2"
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
        class="pointer-events-none rounded-md bg-warning px-1.5 py-0.5 font-mono text-[10px] font-bold tracking-wider text-background"
        title="Development build, with its own data"
        data-od-id="dev-build-badge">DEV</span
      >
    {/if}
  </div>

  <div class="grid gap-0.5 pb-2">
    <Button
      variant="outline"
      class="mb-2 h-8 w-full justify-start gap-2 border-border/60 bg-foreground/[0.03] px-2 text-sm font-normal text-muted-foreground hover:text-foreground"
      aria-label="Search or run a command (Command K)"
      onclick={() => app.requestCommandPalette()}
    >
      <Search class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
      <span class="min-w-0 flex-1 text-left">Search</span>
      <kbd
        class="font-mono text-[10px] text-muted-foreground/80"
        aria-hidden="true">⌘K</kbd
      >
    </Button>

    <Button
      variant="ghost"
      class="h-[30px] w-full justify-start gap-2 px-2 text-sm font-normal text-muted-foreground hover:text-foreground aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-foreground"
      aria-current={app.view === 'homebase' ? 'page' : undefined}
      onclick={() => app.openHomebase()}
    >
      <Layers class="size-4 shrink-0 opacity-80" aria-hidden="true" />
      <span class="flex-1 text-left">Homebase</span>
      <kbd
        class="font-mono text-[10px] text-muted-foreground/80"
        aria-hidden="true">⌘H</kbd
      >
    </Button>

    <Button
      variant="ghost"
      class="h-[30px] w-full justify-start gap-2 px-2 text-sm font-normal text-muted-foreground hover:text-foreground aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-foreground"
      aria-current={app.view === 'todos' ? 'page' : undefined}
      onclick={() => app.openTodos()}
    >
      <ListTodo class="size-4 shrink-0 opacity-80" aria-hidden="true" />
      <span class="flex-1 text-left">TODOs</span>
      {#if todos.items.length > 0}
        <span class="font-mono text-[10px] text-muted-foreground/80"
          >{todos.items.length}</span
        >
      {/if}
    </Button>

    <Button
      variant="ghost"
      class="h-[30px] w-full justify-start gap-2 px-2 text-sm font-normal text-muted-foreground hover:text-foreground"
      onclick={() => app.requestNewWorkspace()}
    >
      <Plus class="size-4 shrink-0 opacity-80" aria-hidden="true" />
      <span class="min-w-0 flex-1 text-left">New workspace</span>
      <kbd
        class="font-mono text-[10px] text-muted-foreground/80"
        aria-hidden="true">⌘N</kbd
      >
    </Button>
  </div>

  <ScrollArea.Root class="sidebar-scroll -mx-2 min-h-0 flex-1 px-2">
    <div class="flex flex-col">
      <div
        class="flex items-center justify-between px-2 pb-1.5 pt-4 text-[10px] font-medium uppercase tracking-wider text-muted-foreground"
        id="side-repo-label"
      >
        <span>Repos</span>
        <span class="font-mono normal-case tracking-normal"
          >{repos.items.length}</span
        >
      </div>
      <ul class="grid list-none gap-px p-0" aria-labelledby="side-repo-label">
        {#if repos.items.length === 0}
          <li class="px-2 py-1.5 text-xs text-muted-foreground">
            No repos yet
          </li>
        {:else}
          {#each repos.items as repo (repo.id)}
            <SidebarRepoRow {repo} />
          {/each}
        {/if}
      </ul>

      <div
        class="flex items-center justify-between px-2 pb-1.5 pt-4 text-[10px] font-medium uppercase tracking-wider text-muted-foreground"
        id="side-ws-label"
      >
        <span>Workspaces</span>
        <span class="font-mono normal-case tracking-normal"
          >{workspaces.items.length}</span
        >
      </div>
      <ul class="grid list-none gap-px p-0" aria-labelledby="side-ws-label">
        {#if workspaces.sidebarItems.length === 0}
          <li class="px-2 py-1.5 text-xs text-muted-foreground">
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

      <div
        class="flex items-center justify-between px-2 pb-1.5 pt-4 text-[10px] font-medium uppercase tracking-wider text-muted-foreground"
        id="side-ag-label"
      >
        <span>Agents</span>
        <span class="font-mono normal-case tracking-normal"
          >{workingAgents} working</span
        >
      </div>
      <ul class="grid list-none gap-px p-0" aria-labelledby="side-ag-label">
        {#if threads.sidebarAgents.length === 0}
          <li class="px-2 py-1.5 text-xs text-muted-foreground">
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

  <div class="flex items-center gap-2 px-2 pt-2">
    <div class="flex min-w-0 flex-1 items-center">
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <span
              {...props}
              tabindex="0"
              class="flex items-center gap-2 rounded-sm text-xs text-muted-foreground"
              aria-label={`Memory used: ${memory.label}`}
            >
              <Cpu class="size-3.5 shrink-0" aria-hidden="true" />
              <span class="font-mono tabular-nums">{memory.label}</span>
            </span>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content side="top">Memory used</Tooltip.Content>
      </Tooltip.Root>
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
