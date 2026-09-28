<script lang="ts">
  import { onMount } from 'svelte'
  import type { Snippet } from 'svelte'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { workspaceUi } from '$lib/state'
  import * as Resizable from '$lib/components/ui/resizable'
  import ChangesPanel from './changes/ChangesPanel.svelte'
  import { cn } from '$lib/utils'

  let {
    workspace,
    thread,
    children,
  }: {
    workspace: Workspace
    thread: Thread
    children: Snippet
  } = $props()

  let wide = $state(true)

  onMount(() => {
    const media = window.matchMedia('(min-width: 1101px)')
    const sync = () => {
      wide = media.matches
    }
    sync()
    media.addEventListener('change', sync)
    return () => media.removeEventListener('change', sync)
  })

  const showPanel = $derived(workspaceUi.changesOpen)
  const split = $derived(showPanel && wide)
  const overlay = $derived(showPanel && !wide)
</script>

<div class={cn('relative flex min-h-0 flex-1 flex-col', split && 'min-h-[420px]')}>
  {#if split}
    <Resizable.PaneGroup direction="horizontal" class="min-h-0 flex-1 rounded-md border border-border/40">
      <Resizable.Pane defaultSize={56} minSize={35} class="min-w-0">
        <div class="flex h-full min-h-0 flex-col overflow-hidden pr-1">
          {@render children()}
        </div>
      </Resizable.Pane>
      <Resizable.Handle withHandle />
      <Resizable.Pane defaultSize={44} minSize={28} maxSize={55} class="min-w-[380px]">
        <ChangesPanel {workspace} {thread} />
      </Resizable.Pane>
    </Resizable.PaneGroup>
  {:else}
    <div class="relative flex min-h-0 flex-1 flex-col">
      {@render children()}
      {#if overlay}
        <ChangesPanel {workspace} {thread} overlay />
      {/if}
    </div>
  {/if}
</div>
