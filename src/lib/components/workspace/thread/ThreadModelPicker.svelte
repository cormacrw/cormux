<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { showToast } from '$lib/feedback/show-toast'
  import { commands } from '$lib/ipc'
  import type { ModelOption, ThreadModels } from '$lib/ipc/bindings'
  import type { Thread } from '$lib/state/threads.svelte'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'

  let { thread }: { thread: Thread } = $props()

  /** The radio value for the engine's own default. */
  const DEFAULT = ''

  let models = $state<ThreadModels | null>(null)
  let open = $state(false)

  const options = $derived.by((): ModelOption[] => {
    if (!models) return []
    const { current, options } = models
    // An ACP engine lists its models only while running; keep a saved pick visible meanwhile.
    if (current && !options.some((option) => option.id === current)) {
      return [...options, { id: current, label: current }]
    }
    return options
  })
  const currentLabel = $derived(
    options.find((option) => option.id === models?.current)?.label ?? 'Default',
  )

  // ACP engines report their models once a session starts, so look again when status changes.
  $effect(() => {
    const threadId = thread.id
    void thread.status
    void load(threadId)
  })

  $effect(() => {
    if (open) void load(thread.id)
  })

  async function load(threadId: string) {
    const result = await commands.threadModels(threadId)
    if (result.status === 'ok' && threadId === thread.id) models = result.data
  }

  async function pick(value: string) {
    if (!models) return
    const threadId = thread.id
    const previous = models.current
    const model = value === DEFAULT ? null : value
    if (model === previous) return
    models = { ...models, current: model }
    const result = await commands.setThreadModel(threadId, model)
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
                : 'Could not change the model',
          },
        ],
      })
    }
  }
</script>

{#if options.length > 0}
  <DropdownMenu.Root bind:open>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button
          {...props}
          type="button"
          variant="ghost"
          size="xs"
          class="text-muted-foreground"
          aria-label={`Model: ${currentLabel}`}
          title="Model for this thread"
          data-od-id="composer-model"
        >
          {currentLabel}
          <ChevronDown class="size-3" aria-hidden="true" />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" side="top" class="w-44">
      <DropdownMenu.Label class="text-xs text-muted-foreground">
        Model
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
