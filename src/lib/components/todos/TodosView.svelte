<script lang="ts">
  import { flip } from 'svelte/animate'
  import { cubicOut } from 'svelte/easing'
  import { fly, slide } from 'svelte/transition'
  import { CLAY_COLORS } from '$lib/clay/identity'
  import CountBead from '$lib/components/clay/CountBead.svelte'
  import { motionMs } from '$lib/motion'
  import { app, todos } from '$lib/state'
  import Plus from '@lucide/svelte/icons/plus'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  const rowFelts = ['bg-butter', 'bg-blush', 'bg-mint', 'bg-lavender']

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

<div class="view-todos relative flex min-h-0 flex-1 flex-col overflow-y-auto">
  <div class="mx-auto flex w-full max-w-[760px] flex-col gap-4 p-8">
    <header class="flex items-center gap-3">
      <h1
        id="todos-title"
        tabindex="-1"
        class="clay-title outline-none"
      >
        TODOs
      </h1>
      <CountBead count={todos.items.length} wiggle={false} />
    </header>

    <div
      class="felt-sm flex flex-col gap-2 p-3"
      role="table"
      aria-labelledby="todos-title"
    >
      <div role="rowgroup" class="flex flex-col gap-2">
        {#each todos.items as todo, index (todo.id)}
          <div
            class="todo-row group/todo grid grid-cols-[44px_minmax(0,1fr)_auto_44px] items-center rounded-[16px_14px_16px_12px] px-2 py-1 {rowFelts[index % rowFelts.length]}"
            role="row"
            data-todo-id={todo.id}
            in:fly={{ y: 8, duration: motionMs(220), easing: cubicOut }}
            out:slide={{ duration: motionMs(180), easing: cubicOut }}
            animate:flip={{ duration: motionMs(200) }}
          >
            <span role="cell" class="flex justify-center">
              <button
                type="button"
                class="grid size-8 place-items-center rounded-full outline-none"
                aria-pressed={todo.pinned}
                aria-label={todo.pinned
                  ? `Unpin ${todo.title} from Homebase`
                  : `Pin ${todo.title} to Homebase`}
                title={todo.pinned ? 'Unpin from Homebase' : 'Pin to Homebase'}
                onclick={() => todos.togglePin(todo.id)}
              >
                <span
                  class="bead size-5 {todo.pinned ? '' : 'opacity-45'}"
                  style="background: {CLAY_COLORS[index % CLAY_COLORS.length]}"
                  aria-hidden="true"
                ></span>
              </button>
            </span>
            <span role="cell" class="min-w-0 truncate py-2.5 pl-1 text-[16px] font-bold"
              >{todo.title}</span
            >
            {#if todo.pinned}
              <span class="pr-2 text-[13px] font-semibold opacity-70" role="cell">Pinned</span>
            {:else}
              <span role="cell"></span>
            {/if}
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
          class="grid grid-cols-[44px_minmax(0,1fr)_44px] items-center rounded-[16px_14px_16px_12px] bg-card/70"
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

    <p class="text-sm text-muted-foreground">
      Press a clay ball to pin a task. Pinned tasks show up on Homebase.
    </p>
  </div>
</div>
