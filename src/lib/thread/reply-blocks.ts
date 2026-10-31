import { Marked } from 'marked'
import { parseChartSpec, type ChartSpec } from './chart-spec'
import { summarizeRun, type RunSummary } from './run-summary'

/** The parts of a finished reply that render as something richer than markdown. */
export type ReplyBlock =
  | { kind: 'text'; text: string }
  | { kind: 'diff'; code: string }
  | { kind: 'mermaid'; code: string }
  | { kind: 'chart'; spec: ChartSpec }
  | { kind: 'json'; value: unknown }
  | { kind: 'run'; code: string; summary: RunSummary }
  | { kind: 'math'; tex: string }

/** Shorter JSON reads fine as a code block. */
const JSON_TREE_MIN_LINES = 12

const LOG_LANGS = ['', 'text', 'txt', 'console', 'log', 'shell', 'sh', 'bash']

const lexer = new Marked({ gfm: true, breaks: true })

function parseJson(code: string): { value: unknown } | null {
  try {
    return { value: JSON.parse(code) as unknown }
  } catch {
    return null
  }
}

function codeBlock(lang: string, code: string): ReplyBlock | null {
  const language = lang.split(/\s/)[0]?.toLowerCase() ?? ''
  if (language === 'diff' || language === 'patch') {
    return { kind: 'diff', code }
  }
  if (language === 'mermaid') return { kind: 'mermaid', code }
  if (language === 'math' || language === 'latex') {
    return { kind: 'math', tex: code }
  }
  if (language === 'chart') {
    const spec = parseChartSpec(code)
    return spec && { kind: 'chart', spec }
  }
  if (language === 'json' && code.split('\n').length >= JSON_TREE_MIN_LINES) {
    const parsed = parseJson(code)
    return parsed && { kind: 'json', value: parsed.value }
  }
  if (LOG_LANGS.includes(language)) {
    const summary = summarizeRun(code)
    return summary && { kind: 'run', code, summary }
  }
  return null
}

/**
 * Splits a reply at the blocks that become components. Only top-level ones count: a fence inside
 * a list stays in the list, so the list isn't cut in two.
 */
export function splitReplyBlocks(text: string): ReplyBlock[] {
  const blocks: ReplyBlock[] = []
  let prose = ''
  const flushProse = () => {
    if (prose) blocks.push({ kind: 'text', text: prose })
    prose = ''
  }
  for (const token of lexer.lexer(text)) {
    const display = /^\s*\$\$([\s\S]+)\$\$\s*$/.exec(token.raw)
    const block =
      token.type === 'code'
        ? codeBlock(String(token.lang ?? ''), String(token.text))
        : token.type === 'paragraph' && display
          ? ({ kind: 'math', tex: display[1]!.trim() } as const)
          : null
    if (!block) {
      prose += token.raw
      continue
    }
    flushProse()
    blocks.push(block)
  }
  flushProse()
  return blocks
}
