<script lang="ts" module>
  import DOMPurify from 'dompurify'

  let renders = 0
</script>

<script lang="ts">
  let { code }: { code: string } = $props()

  let svg = $state<string | null>(null)
  let failed = $state(false)

  // Mermaid is big, so it only loads for a reply that has a diagram. It parses agent text, so
  // it stays in strict mode and its output is sanitised like everything else.
  $effect(() => {
    const source = code
    let stale = false
    void (async () => {
      try {
        const { default: mermaid } = await import('mermaid')
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: document.documentElement.classList.contains('dark')
            ? 'dark'
            : 'neutral',
          themeVariables: { background: 'transparent' },
          fontFamily: 'inherit',
          htmlLabels: false,
          flowchart: { htmlLabels: false },
        })
        renders += 1
        const result = await mermaid.render(`reply-mermaid-${renders}`, source)
        if (stale) return
        svg = DOMPurify.sanitize(result.svg, {
          USE_PROFILES: { svg: true, svgFilters: true },
          // Its colours and label text live in a <style> scoped to the diagram.
          ADD_TAGS: ['style'],
        })
      } catch {
        if (!stale) failed = true
      }
    })()
    return () => {
      stale = true
    }
  })
</script>

{#if failed || svg === null}
  <pre class="reply-mermaid-source"><code>{code}</code></pre>
{:else}
  <figure class="reply-card reply-mermaid" data-od-id="reply-mermaid">
    {@html svg}
  </figure>
{/if}
