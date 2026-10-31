import DOMPurify from 'dompurify'
import hljs from 'highlight.js/lib/core'
import bash from 'highlight.js/lib/languages/bash'
import css from 'highlight.js/lib/languages/css'
import diff from 'highlight.js/lib/languages/diff'
import go from 'highlight.js/lib/languages/go'
import ini from 'highlight.js/lib/languages/ini'
import javascript from 'highlight.js/lib/languages/javascript'
import json from 'highlight.js/lib/languages/json'
import markdown from 'highlight.js/lib/languages/markdown'
import python from 'highlight.js/lib/languages/python'
import rust from 'highlight.js/lib/languages/rust'
import sql from 'highlight.js/lib/languages/sql'
import swift from 'highlight.js/lib/languages/swift'
import typescript from 'highlight.js/lib/languages/typescript'
import xml from 'highlight.js/lib/languages/xml'
import yaml from 'highlight.js/lib/languages/yaml'
import { Marked } from 'marked'
import { markedHighlight } from 'marked-highlight'
import { memoByText } from './memo'

// Only the languages agents usually write, so the bundle doesn't carry all ~190 of them.
for (const [name, language] of Object.entries({
  bash,
  css,
  diff,
  go,
  ini,
  javascript,
  json,
  markdown,
  python,
  rust,
  sql,
  swift,
  typescript,
  xml,
  yaml,
})) {
  hljs.registerLanguage(name, language)
}
hljs.registerAliases(['svelte', 'vue'], { languageName: 'xml' })
hljs.registerAliases(['toml'], { languageName: 'ini' })
hljs.registerAliases(['console', 'shell'], { languageName: 'bash' })

const marked = new Marked(
  // A fence without a known language stays plain; guessing is slow and often wrong on short snippets.
  markedHighlight({
    langPrefix: 'hljs language-',
    highlight: (code, lang) =>
      hljs.getLanguage(lang)
        ? hljs.highlight(code, { language: lang, ignoreIllegals: true }).value
        : code,
  }),
  { gfm: true, breaks: true },
)

const ALLOWED = {
  ALLOWED_TAGS: [
    'p',
    'br',
    'strong',
    'em',
    'code',
    'pre',
    'ul',
    'ol',
    'li',
    'a',
    'blockquote',
    'span',
    'table',
    'thead',
    'tbody',
    'tr',
    'th',
    'td',
  ],
  // `start` keeps a list the agent continues (16., 17., …) from renumbering at 1.
  // `align` is how marked says what a table column's `:---:` asked for.
  ALLOWED_ATTR: ['href', 'rel', 'target', 'start', 'class', 'align'],
}

// A private instance: hooks are per instance, and Mermaid sanitises with the shared one.
const purify = DOMPurify(globalThis.window)

// `class` is only for highlight.js tokens (hljs-title, function_, language-ts), so an agent
// can't pull the app's own Tailwind classes into its reply.
purify.addHook('uponSanitizeAttribute', (_node, data) => {
  if (data.attrName !== 'class') return
  data.attrValue = data.attrValue
    .split(/\s+/)
    .filter((name) => /^(hljs|language-)|_$/.test(name))
    .join(' ')
})

// GitHub's `> [!NOTE]`, and the `> **Warning:**` agents write by habit.
const CALLOUT =
  /<blockquote>\s*<p>(?:\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\](?:<br>)?|<strong>(Note|Tip|Important|Warning|Caution):?<\/strong>:?)\s*/gi

/** Runs on sanitised output and adds only these fixed strings, so the classes are safe. */
function markCallouts(html: string): string {
  return html.replace(CALLOUT, (_match, tag?: string, word?: string) => {
    const kind = (tag ?? word ?? '').toLowerCase()
    const title = kind.charAt(0).toUpperCase() + kind.slice(1)
    return `<blockquote class="callout callout-${kind}"><p><strong class="callout-title">${title}</strong>`
  })
}

function renderSafe(source: string): string {
  const raw = marked.parse(source, { async: false })
  return markCallouts(purify.sanitize(raw, ALLOWED))
}

export const renderSanitizedMarkdown = memoByText(renderSafe, 300)

/**
 * A reply being written, one sanitised piece per top-level block. Every block but the last is
 * finished, so it's rendered once; only the block still growing costs anything per frame.
 */
export function renderMarkdownBlocks(source: string): string[] {
  const blocks = marked.lexer(source).filter((token) => token.type !== 'space')
  return blocks.map((token, index) =>
    index === blocks.length - 1
      ? renderSafe(token.raw)
      : renderSanitizedMarkdown(token.raw),
  )
}

/** ClickUp task descriptions are written as documents, so they also get headings and checklists. */
const RICH = {
  ALLOWED_TAGS: [
    ...ALLOWED.ALLOWED_TAGS,
    'h1',
    'h2',
    'h3',
    'h4',
    'h5',
    'h6',
    'hr',
    'del',
    's',
    'input',
  ],
  ALLOWED_ATTR: [...ALLOWED.ALLOWED_ATTR, 'type', 'checked', 'disabled'],
}

export function renderRichMarkdown(source: string): string {
  const raw = marked.parse(source, { async: false })
  const html = purify.sanitize(raw, RICH)
  // GFM task lists are the only inputs marked emits; anything else is dropped.
  const doc = new DOMParser().parseFromString(html, 'text/html')
  doc.querySelectorAll('input').forEach((input) => {
    if (input.type !== 'checkbox') input.remove()
    else input.disabled = true
  })
  return doc.body.innerHTML
}
