<!--
  The composer's model and effort pickers: a menu over the same kind of per-thread choice.
  ⌘⇧M and ⌘⇧E open them with the current pick focused, ready for the arrow keys and ↵.
-->
<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { commands } from '$lib/ipc'
  import { latestRequests, startsOver } from '$lib/core/owner-scoped'
  import type { ModelOption, ThreadModels } from '$lib/ipc/bindings'
  import { focusPromptAfterPicker } from '$lib/keyboard/focus-home'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import { shortcutLabel } from '$lib/keyboard/shortcuts'
  import {
    claimWorkspacePopover,
    releaseWorkspacePopover,
  } from '$lib/keyboard/popover-registry'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'

  const KINDS = {
    model: {
      name: 'Model',
      shortcut: 'pickModel' as const,
      title: `Model for this thread (${shortcutLabel('pickModel')})`,
      failure: 'Could not change the model',
      readFailure: 'Could not read the models',
      listEngine: commands.engineModels,
      listThread: commands.threadModels,
      setThread: commands.setThreadModel,
    },
    effort: {
      name: 'Effort',
      shortcut: 'pickEffort' as const,
      title: `Effort for this thread (${shortcutLabel('pickEffort')})`,
      failure: 'Could not change the effort',
      readFailure: 'Could not read the effort levels',
      listEngine: commands.engineEfforts,
      listThread: commands.threadEfforts,
      setThread: commands.setThreadEffort,
    },
  }

  let {
    kind,
    threadId,
    draftValue = $bindable(null),
  }: {
    kind: keyof typeof KINDS
    /** Null for a thread that doesn't exist yet: the pick stays in `draftValue` until it's created. */
    threadId: string | null
    draftValue?: string | null
  } = $props()

  /** The radio value for the engine's own default. */
  const DEFAULT = ''

  const config = $derived(KINDS[kind])

  let models = $derived<ThreadModels | null>(startsOver(threadId, null))
  const loads = latestRequests<string | null>()
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
    void load(threadId)
  })

  $effect(() => {
    if (open) void load(threadId)
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

  async function load(forThread: string | null) {
    const isLatest = loads(forThread)
    const result =
      forThread === null
        ? await config.listEngine().then((listed) =>
            listed.status === 'ok'
              ? {
                  status: 'ok' as const,
                  data: { current: draftValue, options: listed.data },
                }
              : listed,
          )
        : await config.listThread(forThread)
    if (!isLatest() || forThread !== threadId) return
    if (result.status === 'ok') models = result.data
    else toastCoreError(result.error, config.readFailure)
  }

  async function pick(value: string) {
    if (!models) return
    const forThread = threadId
    const previous = models.current
    const next = value === DEFAULT ? null : value
    if (next === previous) return
    // A read already under way predates the pick.
    void loads(forThread)
    models = { ...models, current: next }
    if (forThread === null) {
      draftValue = next
      return
    }
    const result = await config.setThread(forThread, next)
    if (result.status === 'error') {
      if (forThread === threadId && models) {
        models = { ...models, current: previous }
      }
      toastCoreError(result.error, config.failure)
    }
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (!shellDialogs.allows(event, config.shortcut)) return
    // A slide-out over the conversation leaves the composer inert.
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
        // Start the arrow keys from the current pick. bits-ui remounts the focus scope a frame
        // after opening and runs this again; by then the arrow keys may have moved on.
        event.preventDefault()
        if (content?.contains(document.activeElement)) return
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
