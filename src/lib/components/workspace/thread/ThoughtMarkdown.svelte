<script lang="ts">
  import { gsap } from 'gsap'
  import { onDestroy } from 'svelte'
  import { prefersReducedMotion } from '$lib/thread/entering'
  import { renderSanitizedMarkdown } from '$lib/thread/sanitize-markdown'
  import { openUrl } from '@tauri-apps/plugin-opener'

  let {
    text,
    cursor = false,
  }: {
    text: string
    /** TextType's ▎ cursor, blinking at the end of the last line while a reply types. */
    cursor?: boolean
  } = $props()

  const html = $derived(renderSanitizedMarkdown(text))

  let root: HTMLDivElement | undefined = $state()
  let cursorEl: HTMLSpanElement | null = null
  let blink: gsap.core.Tween | null = null

  // The markdown is re-rendered on every character, so one cursor node is moved to the end of
  // the newest line each time. Restarting the blink keeps it solid while typing, like an editor.
  $effect(() => {
    void html
    const container = root
    if (!container) return
    if (!cursor) {
      cursorEl?.remove()
      return
    }
    if (!cursorEl) {
      cursorEl = document.createElement('span')
      cursorEl.className = 'type-cursor'
      cursorEl.textContent = '▎'
      cursorEl.setAttribute('aria-hidden', 'true')
    }
    const blocks = container.querySelectorAll('p, li, pre code, blockquote')
    ;(blocks[blocks.length - 1] ?? container).append(cursorEl)
    if (prefersReducedMotion()) return
    if (!blink) {
      blink = gsap.to(cursorEl, {
        opacity: 0,
        duration: 0.5,
        repeat: -1,
        yoyo: true,
        ease: 'power2.inOut',
      })
    } else {
      blink.restart()
    }
  })

  onDestroy(() => blink?.kill())

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
  bind:this={root}
  class="agent-markdown"
  onclick={handleClick}
  role="presentation"
>
  {@html html}
</div>
