<script lang="ts">
  import { flip } from 'svelte/animate'
  import { cubicOut } from 'svelte/easing'
  import { fly, slide } from 'svelte/transition'
  import { motionMs } from '$lib/motion'
  import { app, todos } from '$lib/state'
  import Pin from '@lucide/svelte/icons/pin'
  import Plus from '@lucide/svelte/icons/plus'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  let input: HTMLInputElement | undefined = $state()
  let draft = $state('')

  $effect(() => {
    if (app.focusTarget !== 'todos') return
    void app.focusGeneration
    input?.focus()
  })

  async function add() {
    const title = draft.trim()
    if (!title) return
    // Clear first so the next task can be typed while this one saves.
    draft = ''
    if (!(await todos.add(title)) && !draft) draft = title
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.isComposing) {
      event.preventDefault()
      void add()
    } else if (event.key === 'Escape' && draft) {
      event.preventDefault()
      draft = ''
    }
  }
</script>

<div class="view-todos flex min-h-0 flex-1 flex-col overflow-y-auto">
  <div class="mx-auto flex w-full max-w-[760px] flex-col gap-6 p-6">
    <header class="flex items-center gap-3">
      <h1
        id="todos-title"
        tabindex="-1"
        class="text-2xl font-semibold tracking-tight outline-none"
      >
        TODOs
      </h1>
      <span
        class="rounded-full border border-border bg-muted/50 px-2 py-0.5 font-mono text-xs text-muted-foreground"
        >{todos.items.length}</span
      >
    </header>

    <div
      class="overflow-hidden rounded-xl border border-border bg-card text-card-foreground"
      role="table"
      aria-labelledby="todos-title"
    >
      <div
        class="grid grid-cols-[44px_minmax(0,1fr)_44px] border-b border-border bg-muted/30 text-[11px] font-medium uppercase tracking-wider text-muted-foreground"
        role="row"
      >
        <span class="py-2 text-center" role="columnheader">
          <span class="sr-only">Pinned</span>
          <Pin class="mx-auto size-3" aria-hidden="true" />
        </span>
        <span class="py-2 pl-1" role="columnheader">Task</span>
        <span class="sr-only" role="columnheader">Delete</span>
      </div>

      <div role="rowgroup">
        {#each todos.items as todo (todo.id)}
          <div
            class="todo-row group/todo grid grid-cols-[44px_minmax(0,1fr)_44px] items-center border-b border-border/70 transition-colors [@media(hover:hover)_and_(pointer:fine)]:hover:bg-muted/40"
            role="row"
            data-todo-id={todo.id}
            in:fly={{ y: 8, duration: motionMs(220), easing: cubicOut }}
            out:slide={{ duration: motionMs(180), easing: cubicOut }}
            animate:flip={{ duration: motionMs(200) }}
          >
            <span role="cell" class="flex justify-center">
              <button
                type="button"
                class="flex size-8 items-center justify-center rounded-md outline-none transition-[color,transform] duration-200 focus-visible:ring-3 focus-visible:ring-ring/50 {todo.pinned
                  ? 'text-foreground'
                  : 'text-muted-foreground/50 hover:text-foreground'}"
                aria-pressed={todo.pinned}
                aria-label={todo.pinned
                  ? `Unpin ${todo.title} from Homebase`
                  : `Pin ${todo.title} to Homebase`}
                title={todo.pinned ? 'Unpin from Homebase' : 'Pin to Homebase'}
                onclick={() => todos.togglePin(todo.id)}
              >
                <Pin
                  class="size-4 transition-transform duration-200 {todo.pinned
                    ? 'rotate-0 fill-current'
                    : '-rotate-45'}"
                  aria-hidden="true"
                />
              </button>
            </span>
            <span role="cell" class="min-w-0 truncate py-2.5 pl-1 text-sm"
              >{todo.title}</span
            >
            <span role="cell" class="flex justify-center">
              <button
                type="button"
                class="flex size-8 items-center justify-center rounded-md text-muted-foreground opacity-0 outline-none transition-opacity duration-150 hover:text-destructive focus-visible:opacity-100 focus-visible:ring-3 focus-visible:ring-ring/50 group-hover/todo:opacity-100 [@media(hover:none)]:opacity-100"
                aria-label="Delete {todo.title}"
                title="Delete task"
                onclick={() => todos.remove(todo.id)}
              >
                <Trash2 class="size-4" aria-hidden="true" />
              </button>
            </span>
          </div>
        {/each}

        <div
          class="grid grid-cols-[44px_minmax(0,1fr)_44px] items-center"
          role="row"
        >
          <span role="cell" class="flex justify-center text-muted-foreground">
            <Plus class="size-4" aria-hidden="true" />
          </span>
          <span role="cell" class="min-w-0">
            <input
              bind:this={input}
              bind:value={draft}
              onkeydown={onKeydown}
              data-od-id="todo-add"
              class="w-full bg-transparent py-2.5 pl-1 pr-3 text-sm outline-none placeholder:text-muted-foreground/70"
              placeholder="Add a task"
              aria-label="Add a task"
              autocomplete="off"
            />
          </span>
          <span role="cell"></span>
        </div>
      </div>
    </div>

    {#if todos.items.length === 0}
      <p
        class="text-sm text-muted-foreground"
        in:fly={{ y: 4, duration: motionMs(200) }}
      >
        Type a task and press Enter. Pin one to keep it at the top of Homebase.
      </p>
    {/if}
  </div>
</div>
