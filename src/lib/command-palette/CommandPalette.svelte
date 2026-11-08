<script lang="ts">
  import * as Command from '$lib/components/ui/command/index.js'
  import { Kbd } from '$lib/components/ui/kbd/index.js'
  import {
    filterCommands,
    groupFilteredCommands,
  } from '$lib/command-palette/filter'
  import { buildPaletteCommands } from '$lib/command-palette/registry'
  import { runBang } from '$lib/command-palette/actions'
  import type { PaletteCommand, PaletteIcon } from '$lib/command-palette/types'
  import { bangPreview, paletteBang } from '$lib/bangs/bangs'
  import { palette } from '$lib/command-palette/store.svelte'
  import { settings } from '$lib/settings/store.svelte'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import type { Component } from 'svelte'
  import Download from '@lucide/svelte/icons/download'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import Kanban from '@lucide/svelte/icons/square-kanban'
  import MessageSquare from '@lucide/svelte/icons/message-square'
  import Play from '@lucide/svelte/icons/play'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import Search from '@lucide/svelte/icons/search'
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
  import Square from '@lucide/svelte/icons/square'
  import Zap from '@lucide/svelte/icons/zap'

  const ICONS: Record<PaletteIcon, Component> = {
    bang: Zap,
    findings: Search,
    pull: Download,
    restart: RotateCw,
    run: Play,
    settings: SlidersHorizontal,
    sprint: Kanban,
    stop: Square,
    thread: MessageSquare,
    workspace: GitBranch,
  }

  let query = $state('')
  /** `!name more words`: start a thread with that bang's prompt, the words added. */
  const bangRun = $derived(paletteBang(query, settings.values.bangs))

  const allCommands = $derived(buildPaletteCommands())
  const filtered = $derived(filterCommands(allCommands, query))
  const grouped = $derived(groupFilteredCommands(filtered))

  function onSelect(command: PaletteCommand) {
    // Picking a bang completes it, the same as typing its name: words can follow.
    if (command.kind === 'bang') {
      completeBang(command.label)
      return
    }
    // Closed first (product/features/02-command-palette-and-shortcuts.md), so the command runs with focus back where it was.
    palette.close()
    queueMicrotask(() => command.run())
  }

  function completeBang(label: string) {
    query = `${label} `
    queueMicrotask(() => {
      const field = document.querySelector<HTMLInputElement>(
        '[data-slot="command-input"]',
      )
      field?.focus()
      field?.setSelectionRange(query.length, query.length)
    })
  }

  function startBang() {
    if (!bangRun) return
    const { bang, extra } = bangRun
    palette.close()
    void runBang(bang.id, extra)
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.isComposing) return
    // The field, not `query`: the bound value can trail a fast typist by a tick.
    const text = (event.currentTarget as HTMLInputElement).value
    // Tab completes a partial `!name` to the first bang listed.
    if (event.key === 'Tab' && /^![\w-]*$/.test(text.trim()) && !bangRun) {
      const first = filtered.find((command) => command.kind === 'bang')
      if (first) {
        event.preventDefault()
        event.stopPropagation()
        completeBang(first.label)
        return
      }
    }
    if (bangRun && event.key === 'Enter') {
      event.preventDefault()
      event.stopPropagation()
      startBang()
      return
    }
  }

  $effect(() => {
    if (palette.isOpen) query = ''
  })

  // A dialog opening from a shortcut takes over from the palette.
  $effect(() => {
    if (shellDialogs.anyOpen()) palette.close()
  })
</script>

<Command.Dialog
  bind:open={palette.isOpen}
  shouldFilter={false}
  title="Command palette"
  description="Type a command or workspace name"
  class="glass-pop max-w-[600px] rounded-[16px]! sm:max-w-[600px]"
>
  <Command.Input
    bind:value={query}
    placeholder="Type a command, thread, or !bang…"
    aria-controls="pal-list"
    aria-expanded={palette.isOpen}
    onkeydown={onInputKeydown}
  />
  <Command.List id="pal-list" class="max-h-[min(360px,50vh)]">
    {#if bangRun}
      {@const { bang, extra } = bangRun}
      <Command.Group heading="Bangs">
        <Command.Item
          value="bang-start"
          data-od-id="palette-bang-start"
          onSelect={startBang}
        >
          <Zap class="text-muted-foreground" aria-hidden="true" />
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="truncate"
              >Start a thread with <span class="font-mono">!{bang.name}</span
              ></span
            >
            <span class="truncate text-xs text-muted-foreground">
              {bangPreview(bang.prompt)}{extra ? ` + “${extra}”` : ''}
            </span>
          </span>
        </Command.Item>
      </Command.Group>
    {:else if filtered.length === 0}
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
            {@const Icon = command.icon && ICONS[command.icon]}
            <Command.Item value={command.id} onSelect={() => onSelect(command)}>
              {#if Icon}
                <Icon class="text-muted-foreground" aria-hidden="true" />
              {:else}
                <!-- Holds the icon's place, so labels line up down the list. -->
                <span class="size-4 shrink-0" aria-hidden="true"></span>
              {/if}
              {#if command.subtitle}
                <span class="flex min-w-0 flex-1 flex-col">
                  <span class="truncate">{command.label}</span>
                  <span class="truncate text-xs text-muted-foreground/80">
                    {command.subtitle}
                  </span>
                </span>
              {:else}
                <span class="min-w-0 flex-1 truncate">{command.label}</span>
              {/if}
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
      {bangRun ? 'start thread' : 'run'}
    </span>
  </div>
  <div class="absolute top-3 right-3 hidden sm:flex">
    <Kbd>Esc</Kbd>
  </div>
</Command.Dialog>
