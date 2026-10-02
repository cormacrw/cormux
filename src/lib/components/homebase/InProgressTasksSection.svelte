<script lang="ts">
  import * as Empty from '$lib/components/ui/empty'
  import TaskActions from '$lib/components/sprint/TaskActions.svelte'
  import { formatPoints, statusLabel, taskRef } from '$lib/clickup/board'
  import { app, clickup } from '$lib/state'
  import Kanban from '@lucide/svelte/icons/square-kanban'
  import CircleDashed from '@lucide/svelte/icons/circle-dashed'
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
        class="divide-y rounded-xl border border-border text-sm [&>li]:transition-colors [&>li:hover]:bg-muted/40"
        data-od-id="in-progress-list"
      >
        {#each clickup.inProgress as task (task.id)}
          <li class="group/row flex items-center pr-2">
            <button
              type="button"
              class="flex min-w-0 flex-1 items-center gap-3 py-3 pl-4 text-left outline-none transition-colors focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset"
              onclick={() => app.openSprint(task.id)}
            >
              <span
                class="w-24 shrink-0 truncate font-mono text-xs text-muted-foreground"
                >{taskRef(task)}</span
              >
              <span class="min-w-0 flex-1 truncate font-medium"
                >{task.name}</span
              >
              {#if task.assignees.length > 0}
                <span
                  class="flex shrink-0 -space-x-1.5"
                  title={task.assignees.map((user) => user.username).join(', ')}
                >
                  {#each task.assignees.slice(0, 3) as user (user.id)}
                    <span
                      class="flex size-5 items-center justify-center rounded-full text-[8px] font-semibold text-white ring-2 ring-background"
                      style:background-color={user.color ??
                        'var(--muted-foreground)'}
                      aria-hidden="true">{user.initials}</span
                    >
                  {/each}
                </span>
              {/if}
              <span
                class="inline-flex w-24 shrink-0 items-center gap-1.5 text-xs text-muted-foreground"
              >
                <span
                  class="size-2 rounded-full"
                  style:background-color={task.statusColor ?? 'currentColor'}
                  aria-hidden="true"
                ></span>
                <span class="truncate">{statusLabel(task.status)}</span>
              </span>
              <span class="flex w-16 shrink-0 justify-end">
                {#if task.points == null}
                  <span
                    class="inline-flex items-center gap-1 rounded-full border border-dashed border-warning/80 bg-warning/15 px-1.5 py-px text-[10px] font-semibold text-warning"
                  >
                    <CircleDashed class="size-3" aria-hidden="true" />
                    No pts
                  </span>
                {:else}
                  <span
                    class="inline-flex items-baseline gap-0.5 rounded-full bg-muted px-2 py-px font-mono text-[11px] font-semibold"
                    title="Sprint points"
                    >{formatPoints(task.points)}<span
                      class="text-[9px] font-normal text-muted-foreground"
                      >pts</span
                    ></span
                  >
                {/if}
              </span>
            </button>
            <TaskActions
              {task}
              class="ml-2 opacity-0 transition-opacity group-focus-within/row:opacity-100 [@media(hover:hover)_and_(pointer:fine)]:group-hover/row:opacity-100"
            />
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
              Sprint tasks show up here once they're started.
            {:else if clickup.error}
              {clickup.error}
            {/if}
          </Empty.Description>
        </Empty.Header>
      </Empty.Root>
    {/if}
  </section>
{/if}
