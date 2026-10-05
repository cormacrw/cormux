<script lang="ts">
  import { flip } from 'svelte/animate'
  import { cubicOut } from 'svelte/easing'
  import { scale, slide } from 'svelte/transition'
  import { motionMs } from '$lib/motion'
  import { app, todos } from '$lib/state'

  // Sticky notes take turns in butter and blush, each pinned a little crooked.
  const notes = [
    { felt: 'bg-butter', pin: 'var(--clay-coral)', tilt: '-1.2deg' },
    { felt: 'bg-blush', pin: 'var(--clay-pond)', tilt: '1deg' },
    { felt: 'bg-mint', pin: 'var(--clay-marigold)', tilt: '-0.6deg' },
  ]
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
      class="clay-section"
    >
      Pinned tasks · {todos.pinned.length}
    </h2>
    <div
      class="grid gap-5 [grid-template-columns:repeat(auto-fill,minmax(min(100%,220px),1fr))]"
    >
      {#each todos.pinned as todo, index (todo.id)}
        {@const note = notes[index % notes.length]!}
        <article
          class="group/pin relative flex min-h-[72px] min-w-0 items-stretch rounded-[18px_22px_16px_20px] text-cocoa shadow-lift-2 transition-transform duration-200 ease-[var(--squish)] hover:-translate-y-0.5 {note.felt}"
          style="rotate: {note.tilt}"
          data-pinned-todo={todo.id}
          in:scale={{ start: 0.94, duration: motionMs(220), easing: cubicOut }}
          out:scale={{ start: 0.94, duration: motionMs(160), easing: cubicOut }}
          animate:flip={{ duration: motionMs(220) }}
        >
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center rounded-[18px_22px_16px_20px] py-4 pr-8 pl-6 text-left text-[17px] leading-snug font-bold outline-none"
            title="Open TODOs"
            onclick={() => app.openTodos()}
          >
            <span class="line-clamp-2">{todo.title}</span>
          </button>
          <button
            type="button"
            class="absolute -top-2.5 right-8 grid size-8 place-items-center rounded-full outline-none"
            aria-label="Unpin {todo.title}"
            title="Unpin"
            onclick={() => todos.togglePin(todo.id)}
          >
            <span
              class="bead size-[22px] transition-transform duration-200 ease-[var(--squish)] group-hover/pin:-translate-y-0.5 hover:scale-110"
              style="background: {note.pin}"
              aria-hidden="true"
            ></span>
          </button>
        </article>
      {/each}
    </div>
  </section>
{/if}
