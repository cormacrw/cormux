<!--
  The composer's model and effort pickers: a menu over the same kind of per-thread choice.
  ⌘⇧M and ⌘⇧E open them with the current pick focused, ready for the arrow keys and ↵.
-->
<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { showToast } from '$lib/feedback/show-toast'
  import { commands } from '$lib/ipc'
  import type { ModelOption, ThreadModels } from '$lib/ipc/bindings'
  import { focusPromptAfterPicker } from '$lib/keyboard/focus-home'
  import { isPickerShortcut } from '$lib/keyboard/header-shortcuts'
  import {
    claimWorkspacePopover,
    releaseWorkspacePopover,
  } from '$lib/keyboard/popover-registry'
  import type { Thread } from '$lib/state/threads.svelte'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'

  const KINDS = {
    model: {
      name: 'Model',
      key: 'm',
      title: 'Model for this thread (⌘⇧M)',
      failure: 'Could not change the model',
      listEngine: commands.engineModels,
      listThread: commands.threadModels,
      setThread: commands.setThreadModel,
    },
    effort: {
      name: 'Effort',
      key: 'e',
      title: 'Effort for this thread (⌘⇧E)',
      failure: 'Could not change the effort',
      listEngine: commands.engineEfforts,
      listThread: commands.threadEfforts,
      setThread: commands.setThreadEffort,
    },
  }

  let {
    kind,
    thread,
    draft = false,
    draftValue = $bindable(null),
  }: {
    kind: keyof typeof KINDS
    thread: Thread
    /** A thread that doesn't exist yet: the pick stays in `draftValue` until it's created. */
    draft?: boolean
    draftValue?: string | null
  } = $props()

  /** The radio value for the engine's own default. */
  const DEFAULT = ''

  const config = $derived(KINDS[kind])

  let models = $state<ThreadModels | null>(null)
  let open = $state(false)
  let trigger = $state<HTMLElement | null>(null)
  let content = $state<HTMLElement | null>(null)

  const options = $derived.by((): ModelOption[] => {
    if (!models) return []
    const { current, options } = models
    // Keep a saved pick visible even if it isn't one of the listed models.
    if (current && !options.some((option) => option.id === current)) {
      return [...options, { id: current, label: current }]
    }
    return options
  })
  const pickedLabel = $derived(
    options.find((option) => option.id === models?.current)?.label,
  )
  const currentLabel = $derived(pickedLabel ?? 'Default')
  // Two chips reading "Default" side by side wouldn't say which is which.
  const buttonLabel = $derived(
    pickedLabel ?? (kind === 'effort' ? config.name : 'Default'),
  )

  $effect(() => {
    void load(thread.id)
  })

  $effect(() => {
    if (open) void load(thread.id)
  })

  // Opening one picker from the keyboard closes the other.
  $effect(() => {
    if (!open) return
    const handle = {
      close: () => (open = false),
      restoreFocus: focusPromptAfterPicker,
    }
    claimWorkspacePopover(handle)
    return () => releaseWorkspacePopover(handle)
  })

  async function load(threadId: string) {
    if (draft) {
      const listed = await config.listEngine(thread.engine)
      if (listed.status === 'ok') {
        models = { current: draftValue, options: listed.data }
      }
      return
    }
    const result = await config.listThread(threadId)
    if (result.status === 'ok' && threadId === thread.id) models = result.data
  }

  async function pick(value: string) {
    if (!models) return
    const threadId = thread.id
    const previous = models.current
    const next = value === DEFAULT ? null : value
    if (next === previous) return
    models = { ...models, current: next }
    if (draft) {
      draftValue = next
      return
    }
    const result = await config.setThread(threadId, next)
    if (result.status === 'error') {
      if (threadId === thread.id && models) {
        models = { ...models, current: previous }
      }
      showToast({
        tone: 'bad',
        parts: [
          {
            type: 'text',
            value:
              typeof result.error.message === 'string'
                ? result.error.message
                : config.failure,
          },
        ],
      })
    }
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (!isPickerShortcut(event, config.key)) return
    // Git and Output slide over the conversation and leave the composer inert.
    if (!trigger || trigger.closest('[inert]')) return
    event.preventDefault()
    open = !open
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if options.length > 0}
  <DropdownMenu.Root bind:open>
    <DropdownMenu.Trigger bind:ref={trigger}>
      {#snippet child({ props })}
        <Button
          {...props}
          type="button"
          variant="ghost"
          size="xs"
          class="text-muted-foreground"
          aria-label={`${config.name}: ${currentLabel}`}
          title={config.title}
          data-od-id="composer-{kind}"
        >
          {buttonLabel}
          <ChevronDown class="size-3" aria-hidden="true" />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content
      bind:ref={content}
      align="start"
      side="top"
      class="w-44"
      onOpenAutoFocus={(event) => {
        // Start the arrow keys from the current pick.
        event.preventDefault()
        content?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus()
      }}
      onCloseAutoFocus={(event) => {
        // Back to the prompt rather than the picker's button.
        event.preventDefault()
        focusPromptAfterPicker()
      }}
    >
      <DropdownMenu.Label class="text-xs text-muted-foreground">
        {config.name}
      </DropdownMenu.Label>
      <DropdownMenu.RadioGroup
        value={models?.current ?? DEFAULT}
        onValueChange={(value) => void pick(value)}
      >
        <DropdownMenu.RadioItem value={DEFAULT}>Default</DropdownMenu.RadioItem>
        {#each options as option (option.id)}
          <DropdownMenu.RadioItem value={option.id}>
            {option.label}
          </DropdownMenu.RadioItem>
        {/each}
      </DropdownMenu.RadioGroup>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
{/if}
