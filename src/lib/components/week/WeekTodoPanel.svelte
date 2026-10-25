<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import type { Todo } from '$lib/state/todos.svelte'
  import { todos } from '$lib/state'
  import { todoRef } from '$lib/todos/ref'
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
    /** The status control, in the details under the title. */
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

<!-- Laid out like the ClickUp task panel: an ID chip and actions, the title as
     the heading, details, then the description. -->
<aside
  class="flex h-full min-h-0 flex-col"
  aria-labelledby="week-todo-heading"
  data-od-id="week-todo-panel"
>
  <header
    class="flex items-center gap-2 border-b border-border px-4 py-2.5 text-xs text-muted-foreground"
  >
    <h2
      bind:this={heading}
      id="week-todo-heading"
      tabindex="-1"
      class="rounded-md border border-border bg-muted/50 px-2 py-1 font-mono text-[11px] text-foreground outline-none"
    >
      {todoRef(todo)}
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
    <div class="flex flex-col gap-5 p-4">
      <div class="grid gap-1">
        <input
          id="week-todo-title"
          class="-mx-1.5 w-[calc(100%+12px)] rounded-md bg-transparent px-1.5 py-0.5 text-lg leading-snug font-semibold tracking-tight outline-none hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50"
          aria-label="Title"
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

      {#if children}
        <dl
          class="grid grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 gap-y-3 text-sm"
        >
          <dt class="text-xs text-muted-foreground">Status</dt>
          <dd>{@render children()}</dd>
        </dl>
      {/if}

      <section class="grid gap-2" aria-labelledby="week-todo-desc">
        <h3
          id="week-todo-desc"
          class="text-xs font-medium text-muted-foreground"
        >
          Description
        </h3>
        <textarea
          id="week-todo-description"
          class="field-sizing-content -mx-1.5 min-h-40 w-[calc(100%+12px)] resize-none rounded-md bg-transparent px-1.5 py-1 text-sm leading-relaxed outline-none placeholder:text-muted-foreground hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50"
          rows="8"
          aria-label="Description"
          placeholder="Add notes, links or a checklist…"
          bind:value={description}
          oninput={schedule}
          onblur={save}></textarea>
      </section>
    </div>
  </div>
</aside>
