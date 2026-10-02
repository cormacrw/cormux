<script lang="ts">
  import { gsap } from 'gsap'
  import { mount, onDestroy, unmount } from 'svelte'
  import { prefersReducedMotion } from '$lib/thread/entering'
  import {
    renderRichMarkdown,
    renderSanitizedMarkdown,
  } from '$lib/thread/sanitize-markdown'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import CodeCopyButton from './CodeCopyButton.svelte'

  let {
    text,
    cursor = false,
    rich = false,
  }: {
    text: string
    /** Documents (task descriptions) rather than chat: headings, tables and checklists too. */
    rich?: boolean
    /** TextType's ▎ cursor, blinking at the end of the last line while a reply types. */
    cursor?: boolean
  } = $props()

  const html = $derived(
    rich ? renderRichMarkdown(text) : renderSanitizedMarkdown(text),
  )

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

  // {@html} replaces every node when the text changes, so each render gets fresh copy buttons.
  $effect(() => {
    void html
    const container = root
    if (!container) return
    const buttons = Array.from(container.querySelectorAll('pre'), (pre) =>
      mount(CodeCopyButton, {
        target: pre,
        props: { getText: () => codeText(pre) },
      }),
    )
    return () => buttons.forEach((button) => void unmount(button))
  })

  function codeText(pre: HTMLPreElement) {
    const code = (pre.querySelector('code') ?? pre).cloneNode(true) as Element
    code.querySelector('.type-cursor')?.remove()
    return code.textContent ?? ''
  }

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
