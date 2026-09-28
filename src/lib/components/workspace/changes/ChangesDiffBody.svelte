<script lang="ts">
  import type { DiffFile } from '$lib/ipc/bindings'
  import {
    cachedPreparedFile,
    formatHunkHeader,
    splitDiffRows,
    type PreparedDiffLine,
  } from '$lib/changes/parse-diff'
  import { workspaceUi } from '$lib/state'
  import DiffCode from './DiffCode.svelte'
  import { cn } from '$lib/utils'

  let {
    file,
    scrollEl = $bindable(undefined),
  }: {
    file: DiffFile | undefined
    scrollEl?: HTMLDivElement | undefined
  } = $props()

  const hunks = $derived(file ? cachedPreparedFile(file) : [])
  const mode = $derived(workspaceUi.diffMode)

  const signMeta = {
    '+': { glyph: '+', label: 'Added' },
    '-': { glyph: '−', label: 'Removed' },
    ' ': { glyph: '', label: '' },
  } as const

  function lineClass(kind: PreparedDiffLine['kind']) {
    if (kind === '+') return 'bg-[var(--success)]/10'
    if (kind === '-') return 'bg-destructive/10'
    return ''
  }

  let leftSide: HTMLDivElement | undefined = $state()
  let rightSide: HTMLDivElement | undefined = $state()

  export function bindSplitScroll(node: HTMLDivElement) {
    let lock = false
    const peer = () => (node === leftSide ? rightSide : leftSide)
    const onScroll = () => {
      const other = peer()
      if (!other || lock) return
      lock = true
      other.scrollLeft = node.scrollLeft
      requestAnimationFrame(() => {
        lock = false
      })
    }
    node.addEventListener('scroll', onScroll, { passive: true })
    return {
      destroy() {
        node.removeEventListener('scroll', onScroll)
      },
    }
  }
</script>

{#if file}
  <div
    bind:this={scrollEl}
    role="region"
    tabindex="0"
    aria-label="Proposed changes"
    class={cn(
      'min-h-0 flex-1 overflow-auto font-mono text-xs outline-none',
      mode === 'split' && 'flex flex-col',
    )}
  >
    {#if mode === 'unified'}
      <div class="min-w-max">
        {#each hunks as hunk (hunk.header)}
          <div class="grid grid-cols-[3rem_3rem_1.25rem_1fr] gap-x-1 border-b border-border/30 bg-muted/20 px-2 py-0.5 text-muted-foreground">
            <span></span><span></span><span></span>
            <code>{formatHunkHeader(hunk)}</code>
          </div>
          {#each hunk.lines as line, idx (idx)}
            {@const meta = signMeta[line.kind]}
            <div
              class={cn(
                'grid grid-cols-[3rem_3rem_1.25rem_1fr] gap-x-1 px-2 py-px',
                lineClass(line.kind),
              )}
            >
              <span class="select-none text-right text-muted-foreground">{line.oldLine ?? ''}</span>
              <span class="select-none text-right text-muted-foreground">{line.newLine ?? ''}</span>
              <span class="select-none text-center">
                <span aria-hidden="true">{meta.glyph}</span>
                {#if meta.label}
                  <span class="sr-only">{meta.label}: </span>
                {/if}
              </span>
              <DiffCode text={line.text} path={file.path} wordRange={line.wordRange} />
            </div>
          {/each}
        {/each}
      </div>
    {:else}
      <div class="grid min-h-0 flex-1 grid-cols-2">
        <div
          bind:this={leftSide}
          use:bindSplitScroll
          class="overflow-auto border-r border-border/50"
          aria-label="Before"
        >
          <div class="min-w-max">
            {#each hunks as hunk (hunk.header)}
              <div class="border-b border-border/30 bg-muted/20 px-2 py-0.5 text-muted-foreground">
                <code>{formatHunkHeader(hunk)}</code>
              </div>
              {#each splitDiffRows(hunk.lines) as [left], idx (idx)}
                {#if left}
                  <div class={cn('grid grid-cols-[3rem_1.25rem_1fr] gap-x-1 px-2 py-px', lineClass(left.kind))}>
                    <span class="text-right text-muted-foreground">{left.oldLine ?? left.newLine ?? ''}</span>
                    <span class="text-center" aria-hidden="true">{signMeta[left.kind].glyph}</span>
                    <DiffCode text={left.text} path={file.path} wordRange={left.wordRange} />
                  </div>
                {:else}
                  <div class="grid grid-cols-[3rem_1.25rem_1fr] bg-[repeating-linear-gradient(-45deg,transparent,transparent_6px,color-mix(in_oklch,var(--muted)_40%,transparent)_6px,color-mix(in_oklch,var(--muted)_40%,transparent)_12px)] px-2 py-px">
                    <span></span><span></span><code> </code>
                  </div>
                {/if}
              {/each}
            {/each}
          </div>
        </div>
        <div bind:this={rightSide} use:bindSplitScroll class="overflow-auto" aria-label="After">
          <div class="min-w-max">
            {#each hunks as hunk (hunk.header)}
              <div class="border-b border-border/30 bg-muted/20 px-2 py-0.5 text-muted-foreground">
                <code>{formatHunkHeader(hunk)}</code>
              </div>
              {#each splitDiffRows(hunk.lines) as [, right], idx (idx)}
                {#if right}
                  <div class={cn('grid grid-cols-[3rem_1.25rem_1fr] gap-x-1 px-2 py-px', lineClass(right.kind))}>
                    <span class="text-right text-muted-foreground">{right.newLine ?? right.oldLine ?? ''}</span>
                    <span class="text-center" aria-hidden="true">{signMeta[right.kind].glyph}</span>
                    <DiffCode text={right.text} path={file.path} wordRange={right.wordRange} />
                  </div>
                {:else}
                  <div class="grid grid-cols-[3rem_1.25rem_1fr] bg-[repeating-linear-gradient(-45deg,transparent,transparent_6px,color-mix(in_oklch,var(--muted)_40%,transparent)_6px,color-mix(in_oklch,var(--muted)_40%,transparent)_12px)] px-2 py-px">
                    <span></span><span></span><code> </code>
                  </div>
                {/if}
              {/each}
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}
