<script lang="ts">
  import { plural } from '$lib/format/plural'
  import {
    formatThreadTime,
    formatThreadTimeTitle,
  } from '$lib/thread/thread-time'
  import {
    basename,
    editPath,
    stepLine,
    stepText,
  } from '$lib/thread/timeline/step-text'
  import DiffCounts from './DiffCounts.svelte'
  import type { RunStep } from '$lib/thread/timeline-types'
  import { cn } from '$lib/utils'
  import RunSummaryCard from '../reply/RunSummaryCard.svelte'
  import type { RowContext } from './context'
  import { ICONS } from './icons'
  import { popIn } from './pop-in'
  import {
    agentState,
    iconTone,
    newChips,
    segmentSteps,
    stepChips,
    toolIcon,
  } from './tool-run'

  let { steps, ctx }: { steps: RunStep[]; ctx: RowContext } = $props()

  function editOf(step: RunStep) {
    if (step.kind !== 'tool' || step.role.kind !== 'edit') return null
    return { verb: step.role.verb, path: editPath(step) }
  }
</script>

{#snippet stepExtras(step: RunStep)}
  {#if step.kind === 'tool' && step.run && step.output}
    <li class="ml-[1.125rem] max-w-[42rem] text-sm">
      <RunSummaryCard data={{ summary: step.run, output: step.output }} />
    </li>
  {/if}
  {#if step.kind === 'tool' && step.children?.length}
    <li
      class="ml-1.5 border-l-2 border-border pl-3"
      data-od-id="subagent-steps"
    >
      <ul class="space-y-1">
        {#each step.children as child (child.id)}
          {@const Icon = ICONS[toolIcon(child)]}
          {@const edit = editOf(child)}
          <li
            class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
          >
            <span
              class={cn(
                'size-1.5 shrink-0 rounded-full',
                child.status === 'completed'
                  ? 'bg-success'
                  : child.status === 'failed'
                    ? 'bg-destructive'
                    : 'animate-pulse bg-primary',
              )}
              role="img"
              aria-label={child.status}
            ></span>
            <Icon class="size-3 shrink-0" aria-hidden="true" />
            <span class="truncate"
              >{edit ? `${edit.verb} ${edit.path}` : stepLine(child)}</span
            >
          </li>
        {/each}
      </ul>
    </li>
  {/if}
{/snippet}

<div class="space-y-1.5">
  {#each segmentSteps(steps) as segment (segment.id)}
    <ul class="space-y-1.5">
      {#each segment.steps as step, index (step.id)}
        {@const Icon = ICONS[toolIcon(step)]}
        {@const text = stepText(step)}
        {@const edit = editOf(step)}
        {#if segment.quiet}
          <li
            class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
            title={formatThreadTimeTitle(step.atMs)}
            use:popIn={{ entries: ctx.entries, key: step.id }}
          >
            <Icon
              class={cn(
                'size-3 shrink-0',
                edit ? 'text-primary' : 'text-muted-foreground/80',
              )}
              aria-hidden="true"
            />
            {#if !edit}
              <span class="shrink-0">{text.title}</span>
              {#if text.detail && text.detail !== text.title}
                <span
                  class="truncate font-mono text-muted-foreground/80"
                  title={text.full}>{text.detail}</span
                >
              {/if}
              {#each newChips(step, segment.steps[index - 1], !ctx.workspace) as chip (chip.label)}
                <span class="shrink-0 text-muted-foreground/70"
                  >· {chip.label}</span
                >
              {/each}
              {#if step.kind === 'tool' && step.children?.length}
                <span class="shrink-0 text-muted-foreground/70"
                  >· {plural(step.children.length, 'step')}</span
                >
              {/if}
            {:else if edit.path}
              {@const file = ctx.workspace?.diffFile(edit.path)}
              <span class="shrink-0">{edit.verb}</span>
              <button
                type="button"
                class="truncate font-mono text-primary underline-offset-2 hover:underline"
                title={edit.path}
                aria-label="View diff for {edit.path}"
                disabled={!ctx.workspace}
                onclick={() => ctx.workspace?.revealFile(edit.path)}
              >
                {basename(edit.path)}
              </button>
              {#if file}
                <span class="shrink-0 font-mono"><DiffCounts {file} /></span>
              {/if}
            {:else}
              <span>{edit.verb} a file</span>
            {/if}
          </li>
        {:else}
          {@const state = agentState(step)}
          {@const clock = formatThreadTime(step.atMs)}
          <li
            class="tool-step flex min-w-0 items-start gap-2 text-[0.8rem] text-muted-foreground"
            title={formatThreadTimeTitle(step.atMs)}
            use:popIn={{ entries: ctx.entries, key: step.id }}
          >
            <Icon
              class={cn('mt-[3px] size-3.5 shrink-0', iconTone(step))}
              aria-hidden="true"
            />
            <div class="min-w-0 flex-1">
              <p class="flex min-w-0 flex-wrap items-baseline gap-x-1.5">
                <span class="font-medium text-foreground/90">{text.title}</span>
                {#if text.detail}
                  <span
                    class="min-w-0 truncate font-mono text-[11px] text-muted-foreground/80"
                    >{text.detail}</span
                  >
                {/if}
                {#each stepChips(step, !ctx.workspace) as chip (chip.label)}
                  <span class="text-[11px] text-muted-foreground/75"
                    >· {chip.label}</span
                  >
                {/each}
                {#if state}
                  <span
                    class={cn(
                      'text-[11px]',
                      step.kind === 'tool' && step.status === 'failed'
                        ? 'text-destructive'
                        : 'text-muted-foreground/75',
                    )}
                    data-od-id="agent-state">· {state}</span
                  >
                {/if}
              </p>
              {#if text.full}
                <details class="mt-0.5 text-xs text-muted-foreground">
                  <summary
                    class="cursor-pointer select-none hover:text-foreground"
                    >Details</summary
                  >
                  <pre
                    class="mt-1 max-h-40 overflow-auto rounded-lg bg-muted p-2 font-mono text-(length:--code-font-size) whitespace-pre-wrap">{text.full}</pre>
                </details>
              {/if}
            </div>
            {#if clock}
              <span class="shrink-0 text-[11px] text-muted-foreground/70"
                >{clock}</span
              >
            {/if}
          </li>
        {/if}
        {@render stepExtras(step)}
      {/each}
    </ul>
  {/each}
</div>
