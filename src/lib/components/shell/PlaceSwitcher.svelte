<script lang="ts">
  import Layers from '@lucide/svelte/icons/layers'
  import ListTodo from '@lucide/svelte/icons/list-todo'
  import CalendarDays from '@lucide/svelte/icons/calendar-days'
  import Kanban from '@lucide/svelte/icons/square-kanban'
  import StatusDot from './StatusDot.svelte'
  import { repos, switcher, workspaceRecords, workspaces } from '$lib/state'
  import { statusDotVariantForWorkspace } from '$lib/sidebar/status'
  import { commitPlace } from '$lib/switcher/navigate'
  import { placeKey, type Place } from '$lib/switcher/places'
  import { cn } from '$lib/utils'

  /** Shown only once Ctrl is held a moment, so a quick Ctrl+Tab just switches. */
  const SHOW_AFTER_MS = 120

  let visible = $state(false)
  $effect(() => {
    if (!switcher.open) {
      visible = false
      return
    }
    const timer = setTimeout(() => (visible = true), SHOW_AFTER_MS)
    return () => clearTimeout(timer)
  })

  const PAGES = {
    homebase: { label: 'Homebase', icon: Layers },
    todos: { label: 'TODOs', icon: ListTodo },
    week: { label: 'Week', icon: CalendarDays },
    sprint: { label: 'Sprint', icon: Kanban },
  } as const

  function workspaceDetail(id: string): string {
    const workspace = workspaces.getById(id)
    const repoId = workspaceRecords.getRecord(id)?.repoId
    const repo = repoId ? repos.getById(repoId)?.name : undefined
    return [workspace?.branch, repo].filter(Boolean).join(' · ')
  }

  function label(place: Place): string {
    return place.kind === 'workspace'
      ? (workspaces.getById(place.id)?.name ?? 'Workspace')
      : PAGES[place.kind].label
  }
</script>

{#if switcher.open && visible}
  <div
    class="pointer-events-none fixed inset-0 z-50 flex items-center justify-center bg-background/40"
    data-od-id="place-switcher"
  >
    <ul
      role="listbox"
      aria-label="Switch to"
      class="pointer-events-auto flex max-h-[70vh] w-[26rem] max-w-[90vw] flex-col gap-0.5 overflow-y-auto rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-xl"
    >
      {#each switcher.list as place, index (placeKey(place))}
        {@const selected = index === switcher.index}
        <li
          id="place-option-{index}"
          role="option"
          aria-selected={selected}
          class={cn(
            'flex min-w-0 cursor-default items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm',
            selected && 'bg-muted',
          )}
          onmousedown={(event) => {
            event.preventDefault()
            commitPlace(index)
          }}
          onmouseenter={() => (switcher.index = index)}
        >
          {#if place.kind === 'workspace'}
            {@const workspace = workspaces.getById(place.id)}
            {#if workspace}
              <StatusDot variant={statusDotVariantForWorkspace(workspace)} />
            {/if}
            <span class="truncate font-medium">{label(place)}</span>
            <span
              class="ml-auto truncate font-mono text-xs text-muted-foreground"
              >{workspaceDetail(place.id)}</span
            >
          {:else}
            {@const Icon = PAGES[place.kind].icon}
            <span class="flex size-4 items-center justify-center">
              <Icon class="size-4 text-muted-foreground" aria-hidden="true" />
            </span>
            <span class="truncate font-medium">{label(place)}</span>
          {/if}
        </li>
      {/each}
    </ul>
  </div>
{/if}
<!-- Focus stays put while Ctrl is held, so each step is announced instead. -->
<p class="sr-only" aria-live="polite">
  {#if switcher.open && switcher.list[switcher.index]}
    Switch to {label(switcher.list[switcher.index]!)}
  {/if}
</p>
