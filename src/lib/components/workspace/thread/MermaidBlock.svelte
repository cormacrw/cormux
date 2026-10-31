<script lang="ts" module>
  import DOMPurify from 'dompurify'

  let renders = 0

  // A virtual list unmounts rows off screen; coming back shouldn't redraw or change height.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a cache, nothing renders it
  const drawn = new Map<string, string>()
  const DRAWN_LIMIT = 50

  function drawnKey(code: string) {
    const dark = document.documentElement.classList.contains('dark')
    return `${dark ? 'dark' : 'light'}\u0000${code}`
  }

  function remember(key: string, svg: string) {
    drawn.delete(key)
    drawn.set(key, svg)
    if (drawn.size > DRAWN_LIMIT) {
      drawn.delete(drawn.keys().next().value as string)
    }
  }
</script>

<script lang="ts">
  import { untrack } from 'svelte'

  let { code }: { code: string } = $props()

  let svg = $state<string | null>(
    untrack(() => drawn.get(drawnKey(code)) ?? null),
  )
  let failed = $state(false)

  // Mermaid is big, so it only loads for a reply that has a diagram. It parses agent text, so
  // it stays in strict mode and its output is sanitised like everything else.
  $effect(() => {
    const source = code
    const key = drawnKey(source)
    const known = drawn.get(key)
    if (known) {
      svg = known
      return
    }
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
        const clean = DOMPurify.sanitize(result.svg, {
          USE_PROFILES: { svg: true, svgFilters: true },
          // Its colours and label text live in a <style> scoped to the diagram.
          ADD_TAGS: ['style'],
        })
        remember(key, clean)
        svg = clean
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
