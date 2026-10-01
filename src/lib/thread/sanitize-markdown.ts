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
  ],
  // `start` keeps a list the agent continues (16., 17., …) from renumbering at 1.
  ALLOWED_ATTR: ['href', 'rel', 'target', 'start', 'class'],
}

// `class` is only for highlight.js tokens (hljs-title, function_, language-ts), so an agent
// can't pull the app's own Tailwind classes into its reply.
DOMPurify.addHook('uponSanitizeAttribute', (_node, data) => {
  if (data.attrName !== 'class') return
  data.attrValue = data.attrValue
    .split(/\s+/)
    .filter((name) => /^(hljs|language-)|_$/.test(name))
    .join(' ')
})

export function renderSanitizedMarkdown(source: string): string {
  const raw = marked.parse(source, { async: false })
  return DOMPurify.sanitize(raw, ALLOWED)
}
