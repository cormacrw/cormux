<script lang="ts" module>
  // Lucide's copy and check, as static markup.
  const ICON =
    'xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"'
  const COPY_ICON = `<svg ${ICON}><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>`
  const COPIED_ICON = `<svg class="lucide-check" ${ICON}><path d="M20 6 9 17l-5-5"/></svg>`
  const copiedTimers = new WeakMap<HTMLElement, ReturnType<typeof setTimeout>>()
</script>

<script lang="ts">
  import { untrack } from 'svelte'
  import {
    renderMarkdownBlocks,
    renderRichMarkdown,
    renderSanitizedMarkdown,
  } from '$lib/thread/sanitize-markdown'
  import { enhanceMarkdown } from '$lib/thread/enhance-markdown'
  import { splitLocalMedia, type MediaSegment } from '$lib/thread/local-media'
  import {
    pairBeforeAfter,
    type CompareSegment,
  } from '$lib/thread/media-compare'
  import { splitReplyBlocks, type ReplyBlock } from '$lib/thread/reply-blocks'
  import { prs } from '$lib/state'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import ChartBlock from './ChartBlock.svelte'
  import DiffBlock from './DiffBlock.svelte'
  import JsonTree from './JsonTree.svelte'
  import MathBlock from './MathBlock.svelte'
  import MediaCompare from './MediaCompare.svelte'
  import MermaidBlock from './MermaidBlock.svelte'
  import RunSummaryCard from './RunSummaryCard.svelte'
  import ThreadMedia from './ThreadMedia.svelte'
  import { openDiffForPath } from './open-changes'

  let {
    text,
    cursor = false,
    rich = false,
    media = false,
    workdir,
  }: {
    text: string
    /** Documents (task descriptions) rather than chat: headings and checklists too. */
    rich?: boolean
    /** A ▎ cursor, blinking at the end of the last line while a reply types. */
    cursor?: boolean
    /**
     * Chat replies only: local media files the reply references render inline.
     * Relative ones resolve against `workdir`, the thread's worktree.
     */
    media?: boolean
    workdir?: string
  } = $props()

  // While a reply types out it's all prose: a half-written diagram or chart can't render, and
  // only the finished reply is worth the parse. Local media references become components; the
  // prose between them renders as before.
  type Segment =
    | Exclude<ReplyBlock, { kind: 'text' }>
    | Exclude<MediaSegment, { kind: 'text' }>
    | CompareSegment
    | { kind: 'text'; text: string; html: string }

  const rendered = $derived<Segment[]>(
    (media && !cursor
      ? splitReplyBlocks(text)
      : [{ kind: 'text' as const, text }]
    ).flatMap((block): Segment[] => {
      if (block.kind !== 'text') return [block]
      const segments = media
        ? pairBeforeAfter(splitLocalMedia(block.text))
        : [block]
      return segments.flatMap((segment): Segment[] => {
        if (segment.kind !== 'text') return [segment]
        if (rich) {
          return [{ ...segment, html: renderRichMarkdown(segment.text) }]
        }
        // Mid-typing each block is its own {@html}, so finished ones keep their DOM.
        return cursor
          ? renderMarkdownBlocks(segment.text).map((html) => ({
              ...segment,
              html,
            }))
          : [{ ...segment, html: renderSanitizedMarkdown(segment.text) }]
      })
    }),
  )
  const html = $derived(
    rendered.map((segment) => ('html' in segment ? segment.html : '')).join(''),
  )

  let root: HTMLDivElement | undefined = $state()
  let cursorEl: HTMLSpanElement | null = null

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
    // Only the newest block can hold the end of the text.
    const last = container.lastElementChild
    const newest = last === cursorEl ? last.previousElementSibling : last
    const blocks = newest?.querySelectorAll(
      'p, li, pre code, blockquote, th, td',
    )
    ;(blocks?.[blocks.length - 1] ?? newest ?? container).append(cursorEl)
    for (const animation of cursorEl.getAnimations()) animation.currentTime = 0
  })

  // Chat replies only, and not mid-typing: this walks the whole reply, and it'd run every frame.
  $effect(() => {
    void html
    const container = root
    if (!container || cursor || !media) return
    enhanceMarkdown(container, {
      workdir,
      prs: untrack(() => prs.items),
    })
  })

  // Plain buttons with one click handler below, since a long reply has many code blocks. They
  // wait until the reply is done: mid-typing the blocks are still being replaced.
  $effect(() => {
    void html
    const container = root
    if (!container || cursor) return
    for (const pre of container.querySelectorAll('pre')) {
      if (!pre.querySelector(':scope > .code-copy')) pre.append(copyButton())
    }
  })

  function copyButton() {
    const button = document.createElement('button')
    button.type = 'button'
    button.className =
      'code-copy absolute top-1.5 right-1.5 inline-flex size-6 items-center justify-center rounded bg-muted text-foreground/60 opacity-0 transition hover:text-foreground focus-visible:opacity-100'
    button.ariaLabel = 'Copy code'
    button.title = 'Copy code'
    button.innerHTML = COPY_ICON
    return button
  }

  async function copyCode(button: HTMLButtonElement, pre: HTMLPreElement) {
    try {
      await navigator.clipboard.writeText(codeText(pre))
    } catch {
      return
    }
    button.innerHTML = COPIED_ICON
    clearTimeout(copiedTimers.get(button))
    copiedTimers.set(
      button,
      setTimeout(() => (button.innerHTML = COPY_ICON), 1500),
    )
  }

  function codeText(pre: HTMLPreElement) {
    const code = (pre.querySelector('code') ?? pre).cloneNode(true) as Element
    code.querySelector('.type-cursor')?.remove()
    code.querySelector('.code-copy')?.remove()
    return code.textContent ?? ''
  }

  function handleClick(event: MouseEvent) {
    const clicked = event.target instanceof Element ? event.target : null
    const copy = clicked?.closest<HTMLButtonElement>('.code-copy')
    const copyTarget = copy?.closest('pre')
    if (copy && copyTarget) {
      void copyCode(copy, copyTarget)
      return
    }
    const file = clicked?.closest<HTMLElement>('.file-ref, .file-ref-link')
    if (file?.dataset.path) {
      event.preventDefault()
      const line = file.dataset.line ? Number(file.dataset.line) : null
      openDiffForPath(file.dataset.path, line)
      return
    }
    // Links can wrap <code>/<strong>, so find the anchor from whatever was clicked.
    const target = clicked?.closest('a') ?? null
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
  {#each rendered as segment, index (index)}
    {#if segment.kind === 'text'}
      {@html segment.html}
    {:else if segment.kind === 'media'}
      <ThreadMedia source={segment.source} alt={segment.alt} {workdir} />
    {:else if segment.kind === 'compare'}
      <MediaCompare before={segment.before} after={segment.after} {workdir} />
    {:else if segment.kind === 'diff'}
      <DiffBlock code={segment.code} {workdir} />
    {:else if segment.kind === 'mermaid'}
      <MermaidBlock code={segment.code} />
    {:else if segment.kind === 'chart'}
      <ChartBlock spec={segment.spec} />
    {:else if segment.kind === 'json'}
      <div class="reply-card reply-json" data-od-id="reply-json">
        <JsonTree value={segment.value} />
      </div>
    {:else if segment.kind === 'run'}
      <RunSummaryCard summary={segment.summary} output={segment.code} />
    {:else if segment.kind === 'math'}
      <MathBlock tex={segment.tex} />
    {/if}
  {/each}
</div>
