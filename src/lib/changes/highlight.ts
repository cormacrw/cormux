import { createHighlighter, type Highlighter } from 'shiki'
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript'

const THEME = 'github-dark-default'

const langByExt: Record<string, string> = {
  ts: 'typescript',
  tsx: 'tsx',
  js: 'javascript',
  jsx: 'jsx',
  svelte: 'svelte',
  rs: 'rust',
  py: 'python',
  go: 'go',
  json: 'json',
  md: 'markdown',
  css: 'css',
  html: 'html',
  yml: 'yaml',
  yaml: 'yaml',
  toml: 'toml',
  sh: 'bash',
}

let highlighterPromise: Promise<Highlighter> | null = null
const loadedLangs = new Set<string>()

async function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: [THEME],
      langs: ['plaintext'],
      engine: createJavaScriptRegexEngine(),
    })
  }
  return highlighterPromise
}

export function languageForPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return langByExt[ext] ?? 'plaintext'
}

export async function ensureLanguage(lang: string) {
  if (loadedLangs.has(lang)) return
  const highlighter = await getHighlighter()
  if (highlighter.getLoadedLanguages().includes(lang)) {
    loadedLangs.add(lang)
    return
  }
  if (lang === 'plaintext') {
    loadedLangs.add(lang)
    return
  }
  await highlighter.loadLanguage(lang as 'typescript')
  loadedLangs.add(lang)
}

export async function highlightCode(
  code: string,
  lang: string,
): Promise<string> {
  await ensureLanguage(lang)
  const highlighter = await getHighlighter()
  const html = highlighter.codeToHtml(code || ' ', {
    lang: highlighter.getLoadedLanguages().includes(lang) ? lang : 'plaintext',
    theme: THEME,
  })
  return html
    .replace(/^<pre[^>]*><code[^>]*>/, '')
    .replace(/<\/code><\/pre>\s*$/, '')
}

export function applyWordRangeToHtml(
  html: string,
  range: [number, number] | null | undefined,
): string {
  if (!range) return html
  const plain = stripTags(html)
  const [start, end] = range
  if (start >= end || start < 0 || end > plain.length) return html
  let pos = 0
  let out = ''
  let open = false
  const tokens = html.match(/(<[^>]+>|[^<]+)/g) ?? [html]
  for (const token of tokens) {
    if (token.startsWith('<')) {
      if (open && token.startsWith('</')) {
        out += '</mark>'
        open = false
      }
      out += token
      continue
    }
    const sliceStart = Math.max(0, start - pos)
    const sliceEnd = Math.min(token.length, end - pos)
    if (sliceStart < sliceEnd) {
      out += escapeHtml(token.slice(0, sliceStart))
      if (!open) {
        out += '<mark class="diff-word">'
        open = true
      }
      out += escapeHtml(token.slice(sliceStart, sliceEnd))
      out += escapeHtml(token.slice(sliceEnd))
    } else {
      out += escapeHtml(token)
    }
    pos += token.length
  }
  if (open) out += '</mark>'
  return out
}

function stripTags(html: string): string {
  return html.replace(/<[^>]+>/g, '')
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
}
