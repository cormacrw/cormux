<script lang="ts">
  import { mount, unmount } from 'svelte'
  import {
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
  import CodeCopyButton from './CodeCopyButton.svelte'
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
      return segments.map((segment) =>
        segment.kind === 'text'
          ? {
              ...segment,
              html: rich
                ? renderRichMarkdown(segment.text)
                : renderSanitizedMarkdown(segment.text),
            }
          : segment,
      )
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
    const blocks = container.querySelectorAll(
      'p, li, pre code, blockquote, th, td',
    )
    ;(blocks[blocks.length - 1] ?? container).append(cursorEl)
    for (const animation of cursorEl.getAnimations()) animation.currentTime = 0
  })

  // Chat replies only, and not mid-typing: this walks the whole reply, and it'd run every frame.
  $effect(() => {
    void html
    const container = root
    if (!container || cursor || !media) return
    enhanceMarkdown(container, { workdir, prs: prs.items })
  })

  // {@html} replaces every node when the text changes, so each render gets fresh copy buttons.
  // While a reply types out that's every frame, so they wait until it's done.
  $effect(() => {
    void html
    const container = root
    if (!container || cursor) return
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

  function handleClick(event: MouseEvent) {
    const clicked = event.target instanceof Element ? event.target : null
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
