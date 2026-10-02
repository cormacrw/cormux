<script lang="ts">
  import * as Empty from '$lib/components/ui/empty'
  import { formatPoints, statusLabel } from '$lib/clickup/board'
  import { app, clickup } from '$lib/state'
  import Kanban from '@lucide/svelte/icons/square-kanban'
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert'
</script>

{#if clickup.configured}
  <section
    class="grid gap-4"
    aria-labelledby="in-progress-title"
    data-od-id="home-in-progress"
  >
    <div class="flex flex-wrap items-end justify-between gap-3">
      <h2 id="in-progress-title" class="text-sm font-medium tracking-tight">
        In progress
        <span class="font-mono text-muted-foreground"
          >{clickup.inProgress.length}</span
        >
      </h2>
      {#if clickup.board}
        <button
          type="button"
          class="inline-flex items-center gap-1.5 rounded-md text-xs text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50"
          onclick={() => app.openSprint()}
        >
          <Kanban class="size-3.5" aria-hidden="true" />
          {clickup.board.sprint.name}
        </button>
      {/if}
    </div>

    {#if clickup.inProgress.length > 0}
      <ul
        class="divide-y rounded-xl border border-border text-sm"
        data-od-id="in-progress-list"
      >
        {#each clickup.inProgress as task (task.id)}
          <li>
            <button
              type="button"
              class="flex w-full items-center gap-3 px-4 py-3 text-left outline-none transition-colors hover:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset"
              onclick={() => app.openSprint(task.id)}
            >
              {#if task.customId}
                <span
                  class="w-20 shrink-0 truncate font-mono text-xs text-muted-foreground"
                  >{task.customId}</span
                >
              {/if}
              <span class="min-w-0 flex-1 truncate font-medium"
                >{task.name}</span
              >
              <span
                class="inline-flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground"
              >
                <span
                  class="size-2 rounded-full"
                  style:background-color={task.statusColor ?? 'currentColor'}
                  aria-hidden="true"
                ></span>
                {statusLabel(task.status)}
              </span>
              <span class="flex w-[84px] shrink-0 justify-end">
                {#if task.points == null}
                  <span
                    class="inline-flex shrink-0 items-center gap-1 rounded-md border border-warning/50 bg-warning/15 px-1.5 py-0.5 text-[10px] font-medium text-warning"
                  >
                    <TriangleAlert class="size-3" aria-hidden="true" />
                    No points
                  </span>
                {:else}
                  <span
                    class="inline-flex min-w-6 shrink-0 justify-center rounded-md border border-border bg-muted/60 px-1.5 py-0.5 font-mono text-[11px]"
                    title="Sprint points">{formatPoints(task.points)}</span
                  >
                {/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <Empty.Root class="border border-dashed border-border/80">
        <Empty.Header>
          <Empty.Title>
            {#if !clickup.ready}
              Choose your sprint folder
            {:else if clickup.board}
              Nothing in progress
            {:else if clickup.error}
              Couldn't load the sprint
            {:else}
              Loading the sprint…
            {/if}
          </Empty.Title>
          <Empty.Description>
            {#if !clickup.ready}
              Pick it in Settings › ClickUp to see your tasks here.
            {:else if clickup.board}
              Tasks assigned to you in this sprint show up here once they're
              started.
            {:else if clickup.error}
              {clickup.error}
            {/if}
          </Empty.Description>
        </Empty.Header>
      </Empty.Root>
    {/if}
  </section>
{/if}
