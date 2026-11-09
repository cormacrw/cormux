<!--
  The composer's model and effort pickers: a menu over the same kind of per-thread choice.
  ⌘⇧M and ⌘⇧E open them with the current pick focused, ready for the arrow keys and ↵.
-->
<script lang="ts" module>
  import { commands, type CoreError } from '$lib/ipc'
  import type { Result, ThreadModels } from '$lib/ipc/bindings'

  export type OptionKind = 'model' | 'effort'

  /** Where a picker reads its choices and saves a pick. */
  export type OptionSource = {
    /** Whose pick it is; another starts the picker over. */
    key: string
    list: () => Promise<Result<ThreadModels, CoreError>>
    set: (next: string | null) => Promise<CoreError | null>
  }

  const COMMANDS = {
    model: {
      listEngine: commands.engineModels,
      listThread: commands.threadModels,
      setThread: commands.setThreadModel,
    },
    effort: {
      listEngine: commands.engineEfforts,
      listThread: commands.threadEfforts,
      setThread: commands.setThreadEffort,
    },
  }

  /** A thread's own pick, saved by the core. */
  export function threadOptions(
    kind: OptionKind,
    threadId: string,
  ): OptionSource {
    const { listThread, setThread } = COMMANDS[kind]
    return {
      key: threadId,
      list: () => listThread(threadId),
      set: async (next) => {
        const result = await setThread(threadId, next)
        return result.status === 'error' ? result.error : null
      },
    }
  }

  /** A pick for a thread that doesn't exist yet: it stays with the caller until it's created. */
  export function draftOptions(
    kind: OptionKind,
    pick: { get current(): string | null; set(next: string | null): void },
  ): OptionSource {
    return {
      key: `draft-${kind}`,
      list: async () => {
        const listed = await COMMANDS[kind].listEngine()
        return listed.status === 'ok'
          ? {
              status: 'ok',
              data: { current: pick.current, options: listed.data },
            }
          : listed
      },
      set: async (next) => {
        pick.set(next)
        return null
      },
    }
  }
</script>

<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { latestRequests, startsOver } from '$lib/core/owner-scoped'
  import type { ModelOption } from '$lib/ipc/bindings'
  import { focusPromptAfterPicker } from '$lib/keyboard/focus-home'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import { shortcutLabel } from '$lib/keyboard/shortcuts'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'

  const KINDS = {
    model: {
      name: 'Model',
      shortcut: 'pickModel' as const,
      title: `Model for this thread (${shortcutLabel('pickModel')})`,
      failure: 'Could not change the model',
      readFailure: 'Could not read the models',
    },
    effort: {
      name: 'Effort',
      shortcut: 'pickEffort' as const,
      title: `Effort for this thread (${shortcutLabel('pickEffort')})`,
      failure: 'Could not change the effort',
      readFailure: 'Could not read the effort levels',
    },
  }

  let { kind, source }: { kind: OptionKind; source: OptionSource } = $props()

  /** The radio value for the engine's own default. */
  const DEFAULT = ''

  const config = $derived(KINDS[kind])

  const key = $derived(source.key)
  let models = $derived<ThreadModels | null>(startsOver(key, null))
  const loads = latestRequests<string>()
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
    void load(source)
  })

  $effect(() => {
    if (open) void load(source)
  })

  // Opening one picker from the keyboard closes the other.
  $effect(() => {
    if (!open) return
    const handle = {
      close: () => (open = false),
      restoreFocus: focusPromptAfterPicker,
    }
    keyboardLayers.claimPopover(handle)
    return () => keyboardLayers.releasePopover(handle)
  })

  async function load(from: OptionSource) {
    const isLatest = loads(from.key)
    const result = await from.list()
    if (!isLatest() || from.key !== key) return
    if (result.status === 'ok') models = result.data
    else toastCoreError(result.error, config.readFailure)
  }

  async function pick(value: string) {
    if (!models) return
    const to = source
    const next = value === DEFAULT ? null : value
    if (next === models.current) return
    // A read already under way predates the pick.
    void loads(to.key)
    const error = await to.set(next)
    if (error) toastCoreError(error, config.failure)
    else if (to.key === key && models) models = { ...models, current: next }
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
        bind:value={
          () => models?.current ?? DEFAULT, (value) => void pick(value)
        }
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
