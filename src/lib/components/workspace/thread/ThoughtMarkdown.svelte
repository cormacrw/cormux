<script lang="ts">
  import { renderSanitizedMarkdown } from '$lib/thread/sanitize-markdown'
  import { openUrl } from '@tauri-apps/plugin-opener'

  let { text }: { text: string } = $props()

  const html = $derived(renderSanitizedMarkdown(text))

  function handleClick(event: MouseEvent) {
    // Links can wrap <code>/<strong>, so find the anchor from whatever was clicked.
    const target =
      event.target instanceof Element ? event.target.closest('a') : null
    if (!target) return
    const href = target.getAttribute('href')
    if (!href || href.startsWith('#')) return
    event.preventDefault()
    void openUrl(href)
  }
</script>

<div
  class="prose prose-sm dark:prose-invert max-w-none [&_a]:text-primary [&_p]:my-0 [&_p+p]:mt-2 [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_pre]:text-(length:--code-font-size)"
  onclick={handleClick}
  role="presentation"
>
  {@html html}
</div>
