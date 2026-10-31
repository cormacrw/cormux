import type { PullRequest } from '$lib/state/prs.svelte'
import { findFileRefs, wholeFileRef, worktreePath } from './file-refs'
import { renderMath } from './render-math'

// Blocks mounted as components sit in the same root as the prose, but aren't prose.
const COMPONENTS = '.reply-card, .reply-run-failures, .reply-math, figure'

type Match = { index: number; length: number; make: () => Node }

// `$x^2$`, but not `$HOME and $PATH` or `$5 to $10`: no spaces inside the dollars, no word after.
const INLINE_MATH = /(?<![\w$\\])\$(?=\S)([^$\n]+?)(?<=\S)\$(?![\w$])/g

const GITHUB_LINK =
  /^https:\/\/github\.com\/([^/]+\/[^/]+)\/(pull|commit)\/([0-9a-f]+)\/?$/i

const CHECKS: Record<PullRequest['checks'], string> = {
  pass: 'Checks passing',
  fail: 'Checks failing',
  running: 'Checks running',
  none: '',
}

function fileRefButton(path: string, line: number | null, workdir?: string) {
  const button = document.createElement('button')
  button.type = 'button'
  button.className = 'file-ref'
  button.dataset.path = worktreePath(path, workdir)
  if (line !== null) button.dataset.line = String(line)
  return button
}

function mathSpan(tex: string) {
  const span = document.createElement('span')
  span.textContent = tex
  void renderMath(tex, false).then((html) => (span.innerHTML = html))
  return span
}

function replaceMatches(node: Text, matches: Match[]) {
  const parts = document.createDocumentFragment()
  let last = 0
  const text = node.data
  for (const match of matches.sort((a, b) => a.index - b.index)) {
    if (match.index < last) continue
    parts.append(text.slice(last, match.index), match.make())
    last = match.index + match.length
  }
  parts.append(text.slice(last))
  node.replaceWith(parts)
}

function textMatches(text: string, workdir?: string): Match[] {
  return [
    ...findFileRefs(text).map((ref) => ({
      index: ref.index,
      length: ref.length,
      make: () => {
        const button = fileRefButton(ref.path, ref.line, workdir)
        button.textContent = text.slice(ref.index, ref.index + ref.length)
        return button
      },
    })),
    ...Array.from(text.matchAll(INLINE_MATH), (match) => ({
      index: match.index,
      length: match[0].length,
      make: () => mathSpan(match[1]!),
    })),
  ]
}

function githubCard(link: HTMLAnchorElement, prs: PullRequest[]) {
  const match = GITHUB_LINK.exec(link.href)
  if (!match) return
  const [, repo, type, ref] = match
  const label =
    type === 'pull' ? `${repo}#${ref}` : `${repo}@${ref!.slice(0, 7)}`
  const pr =
    type === 'pull'
      ? prs.find((row) => row.repoFullName === repo && String(row.num) === ref)
      : undefined
  link.className = 'ref-card'
  link.textContent = ''
  const add = (text: string, className: string) => {
    if (!text) return
    const span = document.createElement('span')
    span.className = className
    span.textContent = text
    link.append(span)
  }
  add(label, 'ref-card-id')
  if (!pr) return
  add(pr.title, 'ref-card-title')
  add(CHECKS[pr.checks], `ref-card-checks ref-card-checks-${pr.checks}`)
  if (pr.review === 'approved') add('Approved', 'ref-card-checks')
  if (pr.review === 'changes') add('Changes requested', 'ref-card-checks')
}

/**
 * The inline parts of a finished reply that markdown can't express: clickable file references,
 * PR and commit cards, and math. Runs on the rendered DOM, so sanitising has already happened
 * and everything it adds is built from text nodes, never parsed as HTML.
 */
export function enhanceMarkdown(
  root: HTMLElement,
  options: { workdir?: string; prs: PullRequest[] },
) {
  const { workdir, prs } = options
  for (const code of root.querySelectorAll('code:not(pre code)')) {
    if (code.closest(`a, .file-ref, ${COMPONENTS}`)) continue
    const ref = wholeFileRef(code.textContent ?? '')
    if (!ref) continue
    const button = fileRefButton(ref.path, ref.line, workdir)
    code.replaceWith(button)
    button.append(code)
  }
  for (const link of root.querySelectorAll<HTMLAnchorElement>('a')) {
    const href = link.getAttribute('href') ?? ''
    if (/^[a-z][a-z0-9+.-]*:|^#/i.test(href)) githubCard(link, prs)
    else if (wholeFileRef(href)) {
      const ref = wholeFileRef(href)!
      link.dataset.path = worktreePath(ref.path, workdir)
      if (ref.line !== null) link.dataset.line = String(ref.line)
      link.classList.add('file-ref-link')
    }
  }

  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) =>
      // {@html} tracks its top-level nodes, so those are left in place.
      node.parentElement === root ||
      node.parentElement?.closest(`pre, code, a, button, .katex, ${COMPONENTS}`)
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT,
  })
  const nodes: Text[] = []
  while (walker.nextNode()) nodes.push(walker.currentNode as Text)
  for (const node of nodes) {
    const matches = textMatches(node.data, workdir)
    if (matches.length) replaceMatches(node, matches)
  }
}
