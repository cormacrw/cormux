<script lang="ts" module>
  // The diff viewer brings every highlight.js grammar, so it loads with the first diff.
  let diffView: Promise<typeof import('./ChangesDiffView.svelte')> | undefined
  const loadDiffView = () => (diffView ??= import('./ChangesDiffView.svelte'))
</script>

<script lang="ts">
  import type { Component } from 'svelte'
  import { afterPaint } from '$lib/changes/mount-queue'
  import type { DiffFile } from '$lib/ipc/bindings'

  let {
    workspaceId,
    branch,
    file,
  }: {
    workspaceId: string
    branch: string | null
    file: DiffFile
  } = $props()

  // Highlighting a diff is the slow part of the Git tab, so each one waits until it
  // scrolls near view, then mounts in a frame of its own; it stays mounted after.
  let DiffView =
    $state.raw<
      Component<{ workspaceId: string; branch: string | null; file: DiffFile }>
    >()
  let placeholder: HTMLDivElement | undefined = $state()

  const lineCount = $derived(
    file.hunks.reduce(
      (lines, hunk) => lines + 1 + hunk.body.split('\n').length,
      0,
    ),
  )
  // Roughly the rendered height, so the scrollbar doesn't jump as diffs mount.
  const estimatedHeight = $derived(lineCount * 20)
  const SKELETON_WIDTHS = ['72%', '88%', '54%', '92%', '64%', '80%', '46%']
  const skeletonLines = $derived(
    SKELETON_WIDTHS.slice(0, Math.min(lineCount, SKELETON_WIDTHS.length)),
  )

  $effect(() => {
    if (DiffView || !placeholder) return
    let stopped = false
    let cancel: (() => void) | undefined
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return
        observer.disconnect()
        void loadDiffView().then((view) => {
          if (stopped) return
          cancel = afterPaint(() => (DiffView = view.default))
        })
      },
      // The root has to be the scroller: it clips, so a margin on the viewport does nothing.
      {
        root: placeholder.closest('[data-diff-scroller]'),
        rootMargin: '800px 0px',
      },
    )
    observer.observe(placeholder)
    return () => {
      stopped = true
      observer.disconnect()
      cancel?.()
    }
  })
</script>

{#if DiffView}
  <DiffView {workspaceId} {branch} {file} />
{:else}
  <div
    bind:this={placeholder}
    class="flex flex-col gap-2.5 overflow-hidden px-3 py-3"
    style:height="{estimatedHeight}px"
    aria-hidden="true"
  >
    {#each skeletonLines as width, index (index)}
      <span class="skeleton-bar h-2.5 shrink-0" style:width></span>
    {/each}
  </div>
{/if}
