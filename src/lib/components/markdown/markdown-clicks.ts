import { openUrl } from '@tauri-apps/plugin-opener'
import type { CopyFeedback } from '$lib/primitives/copy-feedback.svelte'

// Lucide's copy and check, as static markup.
const ICON =
  'xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"'
const COPY_ICON = `<svg ${ICON}><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>`
const COPIED_ICON = `<svg class="lucide-check" ${ICON}><path d="M20 6 9 17l-5-5"/></svg>`

let nextKey = 0

function copyButton() {
  const button = document.createElement('button')
  button.type = 'button'
  button.className =
    'code-copy absolute top-1.5 right-1.5 inline-flex size-6 items-center justify-center rounded bg-muted text-foreground/60 opacity-0 transition hover:text-foreground focus-visible:opacity-100'
  button.ariaLabel = 'Copy code'
  button.title = 'Copy code'
  button.dataset.copyKey = `code-${(nextKey += 1)}`
  button.innerHTML = COPY_ICON
  return button
}

/**
 * Plain buttons with one click handler (`copyClicked`), since a long reply has many code blocks.
 * Add them only to finished markdown: mid-typing the blocks are still being replaced.
 */
export function addCopyButtons(container: HTMLElement) {
  for (const pre of container.querySelectorAll('pre')) {
    if (!pre.querySelector(':scope > .code-copy')) pre.append(copyButton())
  }
}

export function showCopied(container: HTMLElement, copied: string | null) {
  for (const button of container.querySelectorAll<HTMLElement>('.code-copy')) {
    const shown = button.dataset.copyKey === copied
    if (shown === (button.dataset.copied === 'true')) continue
    button.dataset.copied = String(shown)
    button.innerHTML = shown ? COPIED_ICON : COPY_ICON
  }
}

function codeText(pre: HTMLPreElement) {
  const code = (pre.querySelector('code') ?? pre).cloneNode(true) as Element
  code.querySelector('.type-cursor')?.remove()
  code.querySelector('.code-copy')?.remove()
  return code.textContent ?? ''
}

/** Copies the code block whose copy button was clicked; false if the click was elsewhere. */
export function copyClicked(event: MouseEvent, clipboard: CopyFeedback) {
  const clicked = event.target instanceof Element ? event.target : null
  const button = clicked?.closest<HTMLButtonElement>('.code-copy')
  const pre = button?.closest('pre')
  if (!button || !pre) return false
  void clipboard.copy(codeText(pre), button.dataset.copyKey)
  return true
}

/** Opens a clicked link in the browser rather than in the app's webview. */
export function openClickedLink(event: MouseEvent) {
  const clicked = event.target instanceof Element ? event.target : null
  // Links can wrap <code>/<strong>, so find the anchor from whatever was clicked.
  const target = clicked?.closest('a') ?? null
  if (!target) return
  const href = target.getAttribute('href')
  if (!href || href.startsWith('#')) return
  event.preventDefault()
  void openUrl(href)
}
