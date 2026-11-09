<script lang="ts" module>
  import { markdownRenderer } from '$lib/markdown/render'

  // Images become their alt text: a document has no worktree to resolve local media in.
  const documents = markdownRenderer()
</script>

<script lang="ts">
  import { onDestroy } from 'svelte'
  import { CopyFeedback } from '$lib/primitives/copy-feedback.svelte'
  import {
    addCopyButtons,
    copyClicked,
    openClickedLink,
    showCopied,
  } from './markdown-clicks'

  /** A document (a task description), so headings and checklists render too. */
  let { text }: { text: string } = $props()

  const html = $derived(documents.rich(text))
  const clipboard = new CopyFeedback()
  let root: HTMLDivElement | undefined = $state()

  $effect(() => {
    void html
    if (root) addCopyButtons(root)
  })

  $effect(() => {
    const copied = clipboard.copied
    if (root) showCopied(root, copied)
  })

  onDestroy(() => clipboard.dispose())

  function handleClick(event: MouseEvent) {
    if (!copyClicked(event, clipboard)) openClickedLink(event)
  }
</script>

<div
  bind:this={root}
  class="agent-markdown"
  onclick={handleClick}
  role="presentation"
>
  {@html html}
</div>
