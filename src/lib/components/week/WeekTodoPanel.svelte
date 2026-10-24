<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import type { Todo } from '$lib/state/todos.svelte'
  import { todos } from '$lib/state'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import X from '@lucide/svelte/icons/x'
  import type { Snippet } from 'svelte'

  /** Typing settles for this long before the title and description save. */
  const SAVE_DELAY = 400

  let {
    todo,
    onClose,
    onDelete,
    children,
  }: {
    /** The panel is keyed by todo id, so a new todo gets a fresh panel and fresh drafts. */
    todo: Todo
    onClose: () => void
    onDelete: () => void
    /** The planning controls, below the fields. */
    children?: Snippet
  } = $props()

  const id = untrack(() => todo.id)
  let title = $state(untrack(() => todo.title))
  let description = $state(untrack(() => todo.description))
  let timer: ReturnType<typeof setTimeout> | undefined
  let heading: HTMLHeadingElement | undefined = $state()

  const blank = $derived(title.trim() === '')

  function save() {
    clearTimeout(timer)
    timer = undefined
    if (blank) return
    void todos.update(id, title, description)
  }

  function schedule() {
    clearTimeout(timer)
    timer = setTimeout(save, SAVE_DELAY)
  }

  function onTitleBlur() {
    if (blank) title = todo.title
    else save()
  }

  onMount(() => heading?.focus({ preventScroll: true }))

  // Closing the pane or opening another card saves what was typed.
  onDestroy(() => {
    if (timer) save()
  })
</script>

<aside
  class="flex h-full min-h-0 flex-col"
  aria-labelledby="week-todo-heading"
  data-od-id="week-todo-panel"
>
  <header
    class="flex items-center gap-2 border-b border-border px-4 py-2.5 text-xs"
  >
    <h2
      bind:this={heading}
      id="week-todo-heading"
      tabindex="-1"
      class="rounded-md bg-accent px-2 py-0.5 font-mono text-[11px] text-accent-foreground outline-none"
    >
      TODO
    </h2>
    <div class="ml-auto flex items-center gap-1">
      <Button
        variant="ghost"
        size="sm"
        class="h-7 gap-1.5 px-2 text-xs text-destructive hover:text-destructive"
        onclick={onDelete}
      >
        <Trash2 class="size-3.5" aria-hidden="true" />
        Delete
      </Button>
      <Button
        variant="ghost"
        size="icon"
        class="size-7"
        aria-label="Close details"
        onclick={onClose}
      >
        <X class="size-4" aria-hidden="true" />
      </Button>
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto">
    <div class="flex flex-col gap-4 p-4">
      <div class="grid gap-1.5">
        <label for="week-todo-title" class="text-xs font-semibold">Title</label>
        <input
          id="week-todo-title"
          class="glass-well h-9 w-full rounded-[10px] px-3 text-sm font-semibold outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          autocomplete="off"
          aria-invalid={blank || undefined}
          bind:value={title}
          oninput={schedule}
          onblur={onTitleBlur}
        />
        {#if blank}
          <p class="text-xs text-destructive" role="alert">
            A TODO needs a title.
          </p>
        {/if}
      </div>
      <div class="grid gap-1.5">
        <label for="week-todo-description" class="text-xs font-semibold"
          >Description</label
        >
        <textarea
          id="week-todo-description"
          class="glass-well min-h-32 w-full resize-y rounded-[10px] px-3 py-2.5 text-[13px] leading-relaxed outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          rows="6"
          placeholder="Add notes, links or a checklist…"
          bind:value={description}
          oninput={schedule}
          onblur={save}></textarea>
      </div>
      {@render children?.()}
    </div>
  </div>
</aside>
