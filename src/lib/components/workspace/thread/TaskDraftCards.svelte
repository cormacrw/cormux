<!--
  Display-only view of work an agent scoped but has not created yet, parsed
  from the reply's <cormux-tasks> block (see the optional cormux-task-cards
  skill): epics with their scope first, tasks under each epic, subtasks
  under each task. Creating stays with the agent in chat.
-->
<script lang="ts">
  import { draftRollup, type TaskDraft } from '$lib/tasks/task-cards-block'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import Check from '@lucide/svelte/icons/check'
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import CircleDashed from '@lucide/svelte/icons/circle-dashed'
  import Flag from '@lucide/svelte/icons/flag'
  import Layers from '@lucide/svelte/icons/layers'
  import Square from '@lucide/svelte/icons/square'
  import X from '@lucide/svelte/icons/x'

  let { drafts }: { drafts: TaskDraft[] } = $props()

  const rollup = $derived(draftRollup(drafts))

  function count(n: number, one: string, many: string) {
    return `${n} ${n === 1 ? one : many}`
  }

  const summary = $derived(
    [
      rollup.epics ? count(rollup.epics, 'epic', 'epics') : null,
      rollup.tasks ? count(rollup.tasks, 'task', 'tasks') : null,
      rollup.subtasks ? count(rollup.subtasks, 'subtask', 'subtasks') : null,
      rollup.points ? `${rollup.points} pts` : null,
    ].filter(Boolean),
  )

  const scopeList = (
    label: string,
    items: string[],
  ): { label: string; items: string[]; included: boolean } => ({
    label,
    items,
    included: label === 'In scope',
  })
</script>

{#snippet meta(draft: TaskDraft)}
  {#if draft.ref || draft.status || draft.priority || draft.points != null || draft.labels.length || draft.url}
    <div class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
      {#if draft.ref}
        <span class="font-mono text-[11px] text-muted-foreground"
          >{draft.ref}</span
        >
      {/if}
      {#if draft.status}
        <span class="rounded-full bg-muted px-2 py-px text-[10px] font-semibold"
          >{draft.status}</span
        >
      {/if}
      {#if draft.priority}
        <span
          class="inline-flex items-center gap-1 text-[11px] text-muted-foreground"
        >
          <Flag class="size-3 shrink-0" aria-hidden="true" />
          {draft.priority}
        </span>
      {/if}
      {#each draft.labels as label (label)}
        <span
          class="rounded-full bg-muted px-2 py-px text-[10px] font-semibold text-muted-foreground"
          >{label}</span
        >
      {/each}
      {#if draft.points != null}
        <span
          class="ml-auto inline-flex shrink-0 items-baseline gap-0.5 rounded-full bg-muted px-2 py-px font-mono text-[11px] font-semibold"
          title="Points"
          >{draft.points}<span
            class="text-[9px] font-normal text-muted-foreground">pts</span
          ></span
        >
      {/if}
      {#if draft.url}
        <button
          type="button"
          class="shrink-0 text-[11px] font-medium text-primary underline-offset-2 hover:underline"
          onclick={() => void openUrl(draft.url!).catch(() => {})}
        >
          Open
        </button>
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet acceptance(items: string[])}
  {#if items.length}
    <ul class="space-y-1.5">
      {#each items as item (item)}
        <li class="flex items-start gap-1.5 text-xs text-muted-foreground">
          <CircleCheck
            class="mt-px size-3 shrink-0 text-success"
            aria-hidden="true"
          />
          <span>{item}</span>
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

{#snippet workNode(draft: TaskDraft)}
  {#if draft.kind === 'subtask'}
    <li class="flex items-start gap-2.5 py-2">
      <Square
        class="mt-0.5 size-3 shrink-0 text-muted-foreground/70"
        aria-hidden="true"
      />
      <div class="min-w-0 flex-1 space-y-1.5">
        <p class="text-xs leading-snug">{draft.title}</p>
        {#if draft.description}
          <p class="text-xs leading-relaxed text-muted-foreground">
            {draft.description}
          </p>
        {/if}
        {@render acceptance(draft.acceptance)}
        {@render meta(draft)}
      </div>
    </li>
  {:else}
    <div class="space-y-2.5 px-1 py-5 first:pt-4 last:pb-1">
      <p class="text-[13px] leading-snug font-medium">{draft.title}</p>
      {#if draft.description}
        <p class="text-xs leading-relaxed text-muted-foreground">
          {draft.description}
        </p>
      {/if}
      {@render acceptance(draft.acceptance)}
      {@render meta(draft)}
      {#if draft.children.length}
        <ul class="mt-3 ml-1.5 border-l border-border pl-4">
          {#each draft.children as child, index (index)}
            {@render workNode(child)}
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
{/snippet}

<div class="mt-5 max-w-[42rem] px-1 pt-2 pb-10" data-od-id="task-draft-cards">
  <div class="border-b border-border pb-4">
    <p class="flex items-center gap-2.5 text-lg font-semibold text-foreground">
      <Layers class="size-5 shrink-0" aria-hidden="true" />
      Scoped work
    </p>
    {#if summary.length || rollup.unestimatedLeaves}
      <p
        class="mt-2 flex flex-wrap items-center gap-x-1.5 gap-y-1 pl-[30px] text-xs text-muted-foreground"
      >
        {#each summary as part (part)}
          <span>{part}</span>{#if part !== summary.at(-1)}<span
              aria-hidden="true">·</span
            >{/if}
        {/each}
        {#if rollup.unestimatedLeaves}
          <span class="inline-flex items-center gap-1 text-warning">
            <CircleDashed class="size-3 shrink-0" aria-hidden="true" />
            {count(rollup.unestimatedLeaves, 'task', 'tasks')} without points
          </span>
        {/if}
      </p>
    {/if}
  </div>

  {#each drafts as draft, index (index)}
    {#if draft.kind === 'epic'}
      <section
        class="mt-6 pt-6 {index > 0 ? 'border-t border-border' : ''}"
        data-od-id="task-draft-epic"
      >
        <div class="flex items-baseline gap-2">
          <span
            class="shrink-0 rounded bg-foreground px-1.5 py-px text-[10px] font-bold tracking-wide text-background uppercase"
            >Epic</span
          >
          <h3 class="text-sm leading-snug font-semibold">{draft.title}</h3>
        </div>
        {#if draft.outcome}
          <p class="mt-3 text-[13px] leading-relaxed">{draft.outcome}</p>
        {/if}
        {#if draft.description}
          <p class="mt-2 text-xs leading-relaxed text-muted-foreground">
            {draft.description}
          </p>
        {/if}

        {#if draft.scope && (draft.scope.in.length || draft.scope.out.length)}
          <div class="mt-5 grid gap-x-8 gap-y-4 sm:grid-cols-2">
            {#each [scopeList('In scope', draft.scope.in), scopeList('Out of scope', draft.scope.out)] as list (list.label)}
              {#if list.items.length}
                <div>
                  <p class="text-[11px] font-semibold text-muted-foreground">
                    {list.label}
                  </p>
                  <ul class="mt-2 space-y-1.5">
                    {#each list.items as item (item)}
                      <li
                        class="flex items-start gap-1.5 text-xs text-muted-foreground"
                      >
                        {#if list.included}
                          <Check
                            class="mt-px size-3 shrink-0 text-success"
                            aria-hidden="true"
                          />
                        {:else}
                          <X
                            class="mt-px size-3 shrink-0 text-muted-foreground/70"
                            aria-hidden="true"
                          />
                        {/if}
                        <span>{item}</span>
                      </li>
                    {/each}
                  </ul>
                </div>
              {/if}
            {/each}
          </div>
        {/if}

        {#if draft.acceptance.length}
          <div class="mt-5">
            <p class="text-[11px] font-semibold text-muted-foreground">
              Done when
            </p>
            <div class="mt-2">{@render acceptance(draft.acceptance)}</div>
          </div>
        {/if}

        {@render meta(draft)}

        {#if draft.children.length}
          <div class="divide-y divide-border/70">
            {#each draft.children as child, index (index)}
              {@render workNode(child)}
            {/each}
          </div>
        {:else}
          <p class="pt-4 text-xs text-warning">
            Not yet broken down into tasks.
          </p>
        {/if}
      </section>
    {:else if index === 0 || drafts[index - 1]?.kind === 'epic'}
      <div class="pt-2 {index === 0 ? 'mt-2' : 'mt-4 border-t border-border'}">
        {@render workNode(draft)}
      </div>
    {:else}
      <div class="border-t border-border/70 pt-1">
        {@render workNode(draft)}
      </div>
    {/if}
  {/each}
</div>
