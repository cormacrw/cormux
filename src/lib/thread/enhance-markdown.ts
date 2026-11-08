import { findFileRefs, wholeFileRef, worktreePath } from './file-refs'
import { renderMath } from './render-math'
import type {
  PrChecksState,
  PrReviewState,
  PullRequestPayload,
} from '$lib/ipc/bindings'

/**
 * Spread on an element that holds a mounted block: blocks sit in the same root as the prose,
 * but aren't prose, so nothing here touches what's inside them.
 */
export const REPLY_BLOCK = { 'data-reply-block': '' }
const BLOCK = '[data-reply-block]'

export function markReplyBlock(element: HTMLElement) {
  element.dataset.replyBlock = ''
}

export function inReplyBlock(element: Element) {
  return element.closest(BLOCK) != null
}

const FILE_REF = '.file-ref, .file-ref-link'

/** The file reference a click landed on, if any. */
export function fileRefAt(
  target: EventTarget | null,
): { path: string; line: number | null } | null {
  if (!(target instanceof Element)) return null
  const ref = target.closest<HTMLElement>(FILE_REF)
  if (!ref?.dataset.path) return null
  const line = ref.dataset.line ? Number(ref.dataset.line) : null
  return { path: ref.dataset.path, line }
}

/** What a PR card shows of a pull request Cormux knows. */
type PullRequestCard = Pick<PullRequestPayload, 'title' | 'checks' | 'review'>

export type FindPullRequest = (
  repoFullName: string,
  num: number,
) => PullRequestCard | undefined

type Match = { index: number; length: number; make: () => Node }

// `$x^2$`, but not `$HOME and $PATH` or `$5 to $10`: no spaces inside the dollars, no word after.
const INLINE_MATH = /(?<![\w$\\])\$(?=\S)([^$\n]+?)(?<=\S)\$(?![\w$])/g

const GITHUB_LINK =
  /^https:\/\/github\.com\/([^/]+\/[^/]+)\/(pull|commit)\/([0-9a-f]+)\/?$/i

const CHECKS: Record<PrChecksState, string> = {
  pass: 'Checks passing',
  fail: 'Checks failing',
  running: 'Checks running',
  none: '',
}

const REVIEW: Record<PrReviewState, string> = {
  approved: 'Approved',
  changes: 'Changes requested',
  required: '',
  draft: '',
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

function textMatches(
  text: string,
  workdir: string | undefined,
  fileRefs: boolean,
): Match[] {
  return [
    ...(fileRefs ? findFileRefs(text) : []).map((ref) => ({
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

function githubCard(link: HTMLAnchorElement, findPr?: FindPullRequest) {
  const match = GITHUB_LINK.exec(link.href)
  if (!match) return
  const [, repo, type, ref] = match
  const label =
    type === 'pull' ? `${repo}#${ref}` : `${repo}@${ref!.slice(0, 7)}`
  const pr = type === 'pull' ? findPr?.(repo!, Number(ref)) : undefined
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
  add(REVIEW[pr.review], 'ref-card-checks')
}

/**
 * The inline parts of a finished reply that markdown can't express: clickable file references
 * (with `fileRefs`, where there are Changes to open them in), PR and commit cards, and math.
 * Runs on the rendered DOM, so sanitising has already happened and everything it adds is built
 * from text nodes, never parsed as HTML.
 */
export function enhanceMarkdown(
  root: HTMLElement,
  options: { workdir?: string; findPr?: FindPullRequest; fileRefs: boolean },
) {
  const { workdir, findPr, fileRefs } = options
  for (const code of fileRefs
    ? root.querySelectorAll('code:not(pre code)')
    : []) {
    if (code.closest(`a, .file-ref, ${BLOCK}`)) continue
    const ref = wholeFileRef(code.textContent ?? '')
    if (!ref) continue
    const button = fileRefButton(ref.path, ref.line, workdir)
    code.replaceWith(button)
    button.append(code)
  }
  for (const link of root.querySelectorAll<HTMLAnchorElement>('a')) {
    if (link.closest(BLOCK)) continue
    const href = link.getAttribute('href') ?? ''
    if (/^[a-z][a-z0-9+.-]*:|^#/i.test(href)) githubCard(link, findPr)
    else if (wholeFileRef(href)) {
      // With nowhere to open it, a link to a file would only try a bogus URL.
      if (!fileRefs) {
        link.removeAttribute('href')
        continue
      }
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
      node.parentElement?.closest(`pre, code, a, button, .katex, ${BLOCK}`)
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT,
  })
  const nodes: Text[] = []
  while (walker.nextNode()) nodes.push(walker.currentNode as Text)
  for (const node of nodes) {
    const matches = textMatches(node.data, workdir, fileRefs)
    if (matches.length) replaceMatches(node, matches)
  }
}
