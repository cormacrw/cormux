<script lang="ts">
  import { onMount } from 'svelte'
  import * as Command from '$lib/components/ui/command/index.js'
  import { Kbd } from '$lib/components/ui/kbd/index.js'
  import {
    filterCommands,
    groupFilteredCommands,
  } from '$lib/command-palette/filter'
  import { buildPaletteCommands } from '$lib/command-palette/registry'
  import { schedulePaletteCommand } from '$lib/command-palette/run-command'
  import type { PaletteCommand } from '$lib/command-palette/types'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
  import { app } from '$lib/state/app.svelte'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'

  let open = $state(false)
  let query = $state('')

  const allCommands = $derived(buildPaletteCommands())
  const filtered = $derived(filterCommands(allCommands, query))
  const grouped = $derived(groupFilteredCommands(filtered))

  function canOpen() {
    return !shellDialogs.blocksCommandPalette()
  }

  function close() {
    open = false
    app.commandPaletteRequested = false
  }

  function openPalette() {
    if (!canOpen()) return
    dismissOpenPopover()
    query = ''
    open = true
  }

  function togglePalette() {
    if (open) {
      close()
      return
    }
    openPalette()
  }

  function onSelect(command: PaletteCommand) {
    schedulePaletteCommand(close, command)
  }

  $effect(() => {
    if (!open) return
    query = ''
  })

  onMount(() => {
    const onRequest = () => togglePalette()
    window.addEventListener('cormux:command-palette', onRequest)
    return () => window.removeEventListener('cormux:command-palette', onRequest)
  })
</script>

<Command.Dialog
  bind:open
  bind:value={query}
  shouldFilter={false}
  title="Command palette"
  description="Type a command or workspace name"
  class="max-w-[600px] sm:max-w-[600px]"
>
  <Command.Input
    placeholder="Type a command or workspace…"
    aria-controls="pal-list"
    aria-expanded={open}
  />
  <Command.List id="pal-list" class="max-h-[min(360px,50vh)]">
    {#if filtered.length === 0}
      <div
        class="px-3 py-6 text-center text-sm text-muted-foreground"
        role="presentation"
      >
        No commands match “{query.trim()}”
      </div>
    {:else}
      {#each grouped as { group, items } (group)}
        <Command.Group heading={group}>
          {#each items as command (command.id)}
            {@const Icon = command.icon}
            <Command.Item value={command.id} onSelect={() => onSelect(command)}>
              {#if Icon}
                <Icon class="text-muted-foreground" aria-hidden="true" />
              {/if}
              <span class="min-w-0 flex-1 truncate">{command.label}</span>
              {#if command.meta}
                <span
                  class="ml-auto max-w-[45%] truncate text-xs text-muted-foreground"
                >
                  {command.meta}
                </span>
              {/if}
              {#if command.kbd}
                <Command.Shortcut>{command.kbd}</Command.Shortcut>
              {/if}
            </Command.Item>
          {/each}
        </Command.Group>
      {/each}
    {/if}
  </Command.List>
  <div
    class="flex items-center justify-between border-t border-border/60 px-3 py-2 text-xs text-muted-foreground"
  >
    <span class="inline-flex items-center gap-1">
      <Kbd>↑</Kbd>
      <Kbd>↓</Kbd>
      navigate
    </span>
    <span class="inline-flex items-center gap-1">
      <Kbd>↵</Kbd>
      run
    </span>
  </div>
  <div class="absolute top-3 right-3 hidden sm:flex">
    <Kbd>Esc</Kbd>
  </div>
</Command.Dialog>
