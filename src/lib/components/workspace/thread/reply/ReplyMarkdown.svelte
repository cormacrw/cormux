<script lang="ts">
  import { mount, onDestroy, unmount, untrack } from 'svelte'
  import {
    addCopyButtons,
    copyClicked,
    openClickedLink,
    showCopied,
  } from '$lib/components/markdown/markdown-clicks'
  import { CopyFeedback } from '$lib/primitives/copy-feedback.svelte'
  import {
    enhanceMarkdown,
    type FindPullRequest,
  } from '$lib/thread/enhance-markdown'
  import {
    INLINE_MEDIA_ATTR,
    mediaKindForSource,
  } from '$lib/thread/reply/media'
  import { parseProse, type ReplySegment } from '$lib/thread/reply/parse'
  import { renderSanitizedMarkdown } from '$lib/thread/sanitize-markdown'
  import { revealInChanges } from '$lib/workspace/open'
  import MediaCompare from './MediaCompare.svelte'
  import ReplyBlock from './ReplyBlock.svelte'
  import ThreadMedia from './ThreadMedia.svelte'

  let {
    text,
    streaming = false,
    workdir,
    findPr,
  }: {
    /** Chat markdown: a reply's prose or a thought. */
    text: string
    /** Still being written: a ▎ cursor blinks at the end of the last line. */
    streaming?: boolean
    /** The thread's worktree; relative file and media references resolve against it. */
    workdir?: string
    /** Pull requests Cormux knows, so a link to one renders as a card with its state. */
    findPr?: FindPullRequest
  } = $props()

  type Part =
    Exclude<ReplySegment, { kind: 'text' }> | { kind: 'html'; html: string }

  // While a reply is written each top-level block is its own {@html}, so finished ones keep
  // their DOM; only the newest is re-rendered per frame.
  const parts = $derived(
    parseProse(text, { streaming }).flatMap((segment, index, all): Part[] => {
      if (segment.kind !== 'text') return [segment]
      if (!streaming) {
        return [{ kind: 'html', html: renderSanitizedMarkdown(segment.raw) }]
      }
      const newest = index === all.length - 1 ? segment.blocks.length - 1 : -1
      return segment.blocks.map((raw, at) => ({
        kind: 'html',
        html: renderSanitizedMarkdown(raw, at === newest),
      }))
    }),
  )
  const html = $derived(
    parts.map((part) => (part.kind === 'html' ? part.html : '')).join(''),
  )

  let root: HTMLDivElement | undefined = $state()
  let cursorEl: HTMLSpanElement | null = null

  // The markdown is re-rendered on every character, so one cursor node is moved to the end of
  // the newest line each time. Restarting the blink keeps it solid while typing, like an editor.
  $effect(() => {
    void html
    const container = root
    if (!container) return
    if (!streaming) {
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

  // Not mid-typing: this walks the whole reply, and it'd run every frame.
  $effect(() => {
    void html
    const container = root
    if (!container || streaming) return
    const mounted = mountInlineMedia(container)
    untrack(() => enhanceMarkdown(container, { workdir, findPr }))
    addCopyButtons(container)
    return () => {
      for (const component of mounted) void unmount(component)
    }
  })

  const clipboard = new CopyFeedback()
  $effect(() => {
    const copied = clipboard.copied
    if (root) showCopied(root, copied)
  })
  onDestroy(() => clipboard.dispose())

  /**
   * Media inside a table, list or quote stays in the prose (cutting it out would break the
   * markdown around it), so it is mounted where the sanitiser left it.
   */
  function mountInlineMedia(container: HTMLElement) {
    const targets = container.querySelectorAll<HTMLElement>(
      `img[${INLINE_MEDIA_ATTR}], a[href]`,
    )
    return Array.from(targets).flatMap((element) => {
      if (element.closest('[data-reply-block]')) return []
      const isImage = element instanceof HTMLImageElement
      const source = isImage
        ? element.getAttribute(INLINE_MEDIA_ATTR)
        : element.getAttribute('href')
      if (!source || !mediaKindForSource(source)) return []
      const alt = isImage ? element.getAttribute('alt') : element.textContent
      const host = document.createElement('span')
      host.className = 'reply-inline-media'
      host.dataset.replyBlock = ''
      element.replaceWith(host)
      return [
        mount(ThreadMedia, {
          target: host,
          props: { source, alt: alt?.trim() ?? '', workdir },
        }),
      ]
    })
  }

  function handleClick(event: MouseEvent) {
    if (copyClicked(event, clipboard)) return
    const clicked = event.target instanceof Element ? event.target : null
    const file = clicked?.closest<HTMLElement>('.file-ref, .file-ref-link')
    if (file?.dataset.path) {
      event.preventDefault()
      const line = file.dataset.line ? Number(file.dataset.line) : null
      revealInChanges(file.dataset.path, line)
      return
    }
    openClickedLink(event)
  }
</script>

<div
  bind:this={root}
  class="agent-markdown"
  onclick={handleClick}
  role="presentation"
>
  {#each parts as part, index (index)}
    {#if part.kind === 'html'}
      {@html part.html}
    {:else if part.kind === 'media'}
      <ThreadMedia source={part.source} alt={part.alt} {workdir} />
    {:else if part.kind === 'compare'}
      <MediaCompare before={part.before} after={part.after} {workdir} />
    {:else}
      <ReplyBlock block={part.block} {workdir} />
    {/if}
  {/each}
</div>
