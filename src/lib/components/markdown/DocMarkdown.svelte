<script lang="ts">
  import { onDestroy } from 'svelte'
  import { CopyFeedback } from '$lib/primitives/copy-feedback.svelte'
  import { renderRichMarkdown } from '$lib/thread/sanitize-markdown'
  import {
    addCopyButtons,
    copyClicked,
    openClickedLink,
    showCopied,
  } from './markdown-clicks'

  /** A document (a task description), so headings and checklists render too. */
  let { text }: { text: string } = $props()

  const html = $derived(renderRichMarkdown(text))
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
