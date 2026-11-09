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
import { memoByText } from '$lib/memo'

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
    'img',
  ],
  // `start` keeps a list the agent continues (16., 17., …) from renumbering at 1.
  // `align` is how marked says what a table column's `:---:` asked for.
  ALLOWED_ATTR: ['href', 'rel', 'target', 'start', 'class', 'align', 'alt'],
}

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

/** Documents (ClickUp task descriptions) also get headings and checklists. */
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

/**
 * The images a renderer keeps: `keep` picks them, and each is marked with `attr` holding its
 * source, for the caller to mount. No image keeps its `src`, so nothing loads from the markup;
 * any other image becomes its alt text.
 */
export type KeptImages = {
  attr: string
  keep: (source: string) => boolean
}

export type MarkdownRenderer = {
  /**
   * Chat markdown as sanitised HTML. Pass `growing` for text still being written, so a string
   * that changes every frame doesn't crowd the cache.
   */
  chat: (source: string, growing?: boolean) => string
  /** A document as sanitised HTML; its checklists are disabled checkboxes. */
  rich: (source: string) => string
}

export function markdownRenderer(images?: KeptImages): MarkdownRenderer {
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

  purify.addHook('uponSanitizeElement', (node) => {
    if (!images || !(node instanceof Element) || node.nodeName !== 'IMG') return
    const source = node.getAttribute('src') ?? ''
    node.removeAttribute(images.attr)
    if (images.keep(source)) node.setAttribute(images.attr, source)
  })

  purify.addHook('afterSanitizeAttributes', (node) => {
    if (node.nodeName !== 'IMG' || !(node instanceof Element)) return
    if (!images || !node.hasAttribute(images.attr)) {
      node.replaceWith(node.getAttribute('alt') ?? '')
    }
  })

  function renderSafe(source: string): string {
    const raw = marked.parse(source, { async: false })
    return markCallouts(purify.sanitize(raw, ALLOWED))
  }

  const renderCached = memoByText(renderSafe, 300)

  return {
    chat: (source, growing = false) =>
      growing ? renderSafe(source) : renderCached(source),
    rich(source) {
      const raw = marked.parse(source, { async: false })
      const html = purify.sanitize(raw, RICH)
      // GFM task lists are the only inputs marked emits; anything else is dropped.
      const doc = new DOMParser().parseFromString(html, 'text/html')
      doc.querySelectorAll('input').forEach((input) => {
        if (input.type !== 'checkbox') input.remove()
        else input.disabled = true
      })
      return doc.body.innerHTML
    },
  }
}
