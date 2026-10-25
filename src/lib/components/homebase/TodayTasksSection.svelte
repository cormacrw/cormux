<!--
  Homebase's Today: the TODOs and sprint tasks planned for today on the Week
  page, as the board's cards. Clicking one opens the task flyout.
-->
<script lang="ts">
  import { flip } from 'svelte/animate'
  import { cubicOut } from 'svelte/easing'
  import { scale } from 'svelte/transition'
  import WeekCard from '$lib/components/week/WeekCard.svelte'
  import { motionMs } from '$lib/motion'
  import { app, clickup, todos, weekPlan } from '$lib/state'
  import { toggleItemDone } from '$lib/week/actions'
  import { weekItems, type WeekItem } from '$lib/week/items'

  const today = $derived(
    weekItems(
      todos.items,
      clickup.ready ? (clickup.mine ?? null) : null,
      weekPlan.rows,
    ).filter((item) => item.plannedOn === weekPlan.today),
  )
  const done = $derived(today.filter((item) => item.done).length)
  const selection = $derived(weekPlan.selected())

  function open(item: WeekItem) {
    const same = selection?.kind === item.kind && selection.id === item.id
    weekPlan.select(same ? null : { kind: item.kind, id: item.id })
  }
</script>

<section
  class="grid gap-4"
  aria-labelledby="today-tasks-title"
  data-od-id="home-today"
>
  <h2
    id="today-tasks-title"
    class="flex items-center gap-2 text-sm font-medium tracking-tight"
  >
    Today
    {#if today.length}
      <span class="font-mono text-xs text-muted-foreground"
        >{done} of {today.length} done</span
      >
    {/if}
  </h2>
  {#if today.length}
    <div
      class="grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(min(100%,260px),1fr))]"
      role="list"
    >
      {#each today as item (item.key)}
        <div
          role="listitem"
          in:scale={{ start: 0.96, duration: motionMs(200), easing: cubicOut }}
          animate:flip={{ duration: motionMs(220) }}
        >
          <WeekCard
            {item}
            draggable={false}
            selected={selection?.kind === item.kind && selection.id === item.id}
            onOpen={() => open(item)}
            onToggleDone={() => toggleItemDone(item)}
          />
        </div>
      {/each}
    </div>
  {:else}
    <p class="text-sm text-muted-foreground">
      Nothing planned for today.
      <button
        type="button"
        class="font-medium text-foreground underline-offset-2 hover:underline"
        onclick={() => app.openWeek()}>Plan your week</button
      >
    </p>
  {/if}
</section>
