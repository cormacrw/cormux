<script lang="ts">
  import {
    applyWordRangeToHtml,
    highlightCode,
    languageForPath,
  } from '$lib/changes/highlight'

  let {
    text,
    path,
    wordRange = null,
  }: {
    text: string
    path: string
    wordRange?: [number, number] | null
  } = $props()

  let html = $state('')

  $effect(() => {
    const lang = languageForPath(path)
    void (async () => {
      const highlighted = await highlightCode(text, lang)
      html = applyWordRangeToHtml(highlighted, wordRange)
    })()
  })
</script>

<code class="diff-code whitespace-pre">{@html html || ' '}</code>

<style>
  :global(.diff-code .diff-word) {
    background: color-mix(in oklch, var(--warning) 35%, transparent);
    border-radius: 2px;
  }
  :global(.diff-code span) {
    font-family: var(--font-mono);
  }
</style>
