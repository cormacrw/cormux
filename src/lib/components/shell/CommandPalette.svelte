<script lang="ts">
  import { onMount } from 'svelte'
  import * as Command from '$lib/components/ui/command/index.js'
  import { Kbd } from '$lib/components/ui/kbd/index.js'
  import {
    filterCommands,
    groupFilteredCommands,
  } from '$lib/command-palette/filter'
  import { buildPaletteCommands } from '$lib/command-palette/registry'
  import { runBang } from '$lib/command-palette/actions'
  import { schedulePaletteCommand } from '$lib/command-palette/run-command'
  import type { PaletteCommand } from '$lib/command-palette/types'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
  import { bangPreview, paletteBang } from '$lib/settings/bangs'
  import { app } from '$lib/state/app.svelte'
  import { settings } from '$lib/state/settings.svelte'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import Zap from '@lucide/svelte/icons/zap'

  let open = $state(false)
  let query = $state('')
  /** `!name more words`: start a thread with that bang's prompt, the words added. */
  const bangRun = $derived(paletteBang(query, settings.bangs))

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
    // Picking a bang completes it, the same as typing its name: words can follow.
    if (command.id.startsWith('bang-run-')) {
      completeBang(command.label)
      return
    }
    schedulePaletteCommand(close, command)
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
    close()
    void runBang(bang.id, extra)
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.isComposing) return
    // The field, not `query`: the bound value can trail a fast typist by a tick.
    const text = (event.currentTarget as HTMLInputElement).value
    // Tab completes a partial `!name` to the first bang listed.
    if (event.key === 'Tab' && /^![\w-]*$/.test(text.trim()) && !bangRun) {
      const first = filtered.find((command) =>
        command.id.startsWith('bang-run-'),
      )
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
    if (!open) return
    query = ''
  })

  onMount(() => {
    const onRequest = () => togglePalette()
    const onClose = () => close()
    window.addEventListener('cormux:command-palette', onRequest)
    window.addEventListener('cormux:close-palette', onClose)
    return () => {
      window.removeEventListener('cormux:command-palette', onRequest)
      window.removeEventListener('cormux:close-palette', onClose)
    }
  })
</script>

<Command.Dialog
  bind:open
  shouldFilter={false}
  title="Command palette"
  description="Type a command or workspace name"
  class="glass-pop max-w-[600px] rounded-[16px]! sm:max-w-[600px]"
>
  <Command.Input
    bind:value={query}
    placeholder="Type a command, thread, or !bang…"
    aria-controls="pal-list"
    aria-expanded={open}
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
            {@const Icon = command.icon}
            <Command.Item value={command.id} onSelect={() => onSelect(command)}>
              {#if Icon}
                <Icon class="text-muted-foreground" aria-hidden="true" />
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
