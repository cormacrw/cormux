<script lang="ts">
  import type { DiffFile } from '$lib/ipc/bindings'
  import ChangesDiffView from './ChangesDiffView.svelte'

  let {
    workspaceId,
    branch,
    file,
  }: {
    workspaceId: string
    branch: string | null
    file: DiffFile
  } = $props()

  // Highlighting every diff at once is what made the Git tab slow to open, so each
  // waits until it scrolls near view, then stays mounted.
  let shown = $state(false)
  let placeholder: HTMLDivElement | undefined = $state()

  // Roughly the rendered height, so the scrollbar doesn't jump as diffs mount.
  const estimatedHeight = $derived(
    file.hunks.reduce(
      (lines, hunk) => lines + 1 + hunk.body.split('\n').length,
      0,
    ) * 20,
  )

  $effect(() => {
    if (shown || !placeholder) return
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          shown = true
          observer.disconnect()
        }
      },
      { rootMargin: '800px 0px' },
    )
    observer.observe(placeholder)
    return () => observer.disconnect()
  })
</script>

{#if shown}
  <ChangesDiffView {workspaceId} {branch} {file} />
{:else}
  <div
    bind:this={placeholder}
    style:height="{estimatedHeight}px"
    aria-hidden="true"
  ></div>
{/if}
