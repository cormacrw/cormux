<script lang="ts">
  import { onMount } from 'svelte'
  import * as Command from '$lib/components/ui/command/index.js'
  import { Kbd } from '$lib/components/ui/kbd/index.js'
  import {
    filterCommands,
    groupFilteredCommands,
  } from '$lib/command-palette/filter'
  import { ADD_TODO_COMMAND_ID } from '$lib/command-palette/build-registry'
  import { buildPaletteCommands } from '$lib/command-palette/registry'
  import {
    isTodoTrigger,
    TODO_TRIGGER,
    todoTitleFromQuery,
  } from '$lib/command-palette/todo-mode'
  import { runBang } from '$lib/command-palette/actions'
  import { schedulePaletteCommand } from '$lib/command-palette/run-command'
  import type { PaletteCommand } from '$lib/command-palette/types'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
  import { motionMs } from '$lib/motion'
  import { bangPreview, paletteBang } from '$lib/settings/bangs'
  import { app } from '$lib/state/app.svelte'
  import { settings } from '$lib/state/settings.svelte'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import { todos } from '$lib/state/todos.svelte'
  import { showToast } from '$lib/feedback/show-toast'
  import { cubicOut } from 'svelte/easing'
  import { scale } from 'svelte/transition'
  import Plus from '@lucide/svelte/icons/plus'
  import Zap from '@lucide/svelte/icons/zap'

  let open = $state(false)
  let query = $state('')
  /** "todo" was turned into a chip: the input now holds a task title. */
  let todoMode = $state(false)
  const chipText = $derived(query.trim())
  /** `!name more words`: start a scratch with that bang's prompt, the words added. */
  const bangRun = $derived(todoMode ? null : paletteBang(query, settings.bangs))

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
    if (command.id === ADD_TODO_COMMAND_ID) {
      enterTodoMode('')
      return
    }
    // Picking a bang completes it, the same as typing its name: words can follow.
    if (command.id.startsWith('bang-run-')) {
      completeBang(command.label)
      return
    }
    schedulePaletteCommand(close, command)
  }

  function enterTodoMode(title: string) {
    todoMode = true
    query = title
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

  async function addTodo(text: string) {
    const title = text.trim()
    if (!title) return
    close()
    if (await todos.add(title)) {
      showToast({
        tone: 'ok',
        parts: [{ type: 'text', value: `Added “${title}” to TODOs` }],
      })
    }
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.isComposing) return
    // The field, not `query`: the bound value can trail a fast typist by a tick.
    const text = (event.currentTarget as HTMLInputElement).value
    // Stop the keys here so the command list doesn't also act on them.
    if (todoMode) {
      if (event.key === 'Enter') {
        event.preventDefault()
        event.stopPropagation()
        void addTodo(text)
      } else if (event.key === 'Backspace' && text === '') {
        event.preventDefault()
        todoMode = false
        query = TODO_TRIGGER
      }
      return
    }
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
    if (
      (event.key === ' ' || event.key === 'Tab' || event.key === 'Enter') &&
      isTodoTrigger(text)
    ) {
      event.preventDefault()
      event.stopPropagation()
      enterTodoMode('')
      return
    }
  }

  // A pasted "todo <title>" never presses the trigger key.
  $effect(() => {
    if (todoMode) return
    const title = todoTitleFromQuery(query)
    if (title != null) enterTodoMode(title)
  })

  $effect(() => {
    if (!open) return
    query = ''
    todoMode = false
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
    placeholder={todoMode
      ? 'Add a task…'
      : 'Type a command, workspace, or !bang…'}
    aria-label={todoMode ? 'Task title' : undefined}
    aria-controls="pal-list"
    aria-expanded={open}
    onkeydown={onInputKeydown}
  >
    {#snippet leading()}
      {#if todoMode}
        <span
          class="rounded-md bg-foreground/10 px-1.5 py-0.5 text-xs font-medium text-foreground"
          data-od-id="palette-todo-chip"
          in:scale={{ start: 0.8, duration: motionMs(160), easing: cubicOut }}
          >TODO</span
        >
      {/if}
    {/snippet}
  </Command.Input>
  <Command.List id="pal-list" class="max-h-[min(360px,50vh)]">
    {#if todoMode}
      <Command.Group heading="TODOs">
        <Command.Item
          value="todo-add"
          disabled={!chipText}
          onSelect={() => void addTodo(query)}
        >
          <Plus class="text-muted-foreground" aria-hidden="true" />
          <span class="min-w-0 flex-1 truncate">
            {chipText ? `Add “${chipText}”` : 'Type a task, then press Enter'}
          </span>
        </Command.Item>
      </Command.Group>
    {:else if bangRun}
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
              >Start a scratch with <span class="font-mono">!{bang.name}</span
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
      {todoMode ? 'add task' : bangRun ? 'start scratch' : 'run'}
    </span>
  </div>
  <div class="absolute top-3 right-3 hidden sm:flex">
    <Kbd>Esc</Kbd>
  </div>
</Command.Dialog>
