<script lang="ts">
  import { flip } from 'svelte/animate'
  import { cubicOut } from 'svelte/easing'
  import { scale, slide } from 'svelte/transition'
  import { motionMs } from '$lib/motion'
  import { app, todos } from '$lib/state'
  import Pin from '@lucide/svelte/icons/pin'
</script>

{#if todos.pinned.length > 0}
  <section
    class="grid gap-4"
    aria-labelledby="pinned-todos-title"
    data-od-id="home-pinned-todos"
    transition:slide={{ duration: motionMs(220), easing: cubicOut }}
  >
    <h2
      id="pinned-todos-title"
      class="flex items-center gap-2 text-sm font-medium tracking-tight"
    >
      Pinned tasks
      <span
        class="rounded-full border border-border bg-muted/50 px-2 py-0.5 font-mono text-xs text-muted-foreground"
        >{todos.pinned.length}</span
      >
    </h2>
    <div
      class="grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(min(100%,240px),1fr))]"
    >
      {#each todos.pinned as todo (todo.id)}
        <article
          class="group/pin flex min-h-[52px] min-w-0 items-stretch rounded-xl border border-border bg-card text-card-foreground transition-colors [@media(hover:hover)_and_(pointer:fine)]:hover:border-foreground/25 [@media(hover:hover)_and_(pointer:fine)]:hover:bg-muted/40"
          data-pinned-todo={todo.id}
          in:scale={{ start: 0.94, duration: motionMs(220), easing: cubicOut }}
          out:scale={{ start: 0.94, duration: motionMs(160), easing: cubicOut }}
          animate:flip={{ duration: motionMs(220) }}
        >
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center rounded-l-xl py-2 pl-4 pr-2 text-left text-sm font-medium outline-none focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset"
            title="Open TODOs"
            onclick={() => app.openTodos()}
          >
            <span class="line-clamp-2">{todo.title}</span>
          </button>
          <button
            type="button"
            class="mr-1 flex size-11 shrink-0 items-center justify-center self-center rounded-lg text-foreground/80 outline-none transition-colors hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset"
            aria-label="Unpin {todo.title}"
            title="Unpin"
            onclick={() => todos.togglePin(todo.id)}
          >
            <Pin
              class="size-4 fill-current transition-transform duration-200 group-hover/pin:-rotate-12"
              aria-hidden="true"
            />
          </button>
        </article>
      {/each}
    </div>
  </section>
{/if}
