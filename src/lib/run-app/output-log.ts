import type { LogEntry, LogKind } from '$lib/ipc/bindings'

/** Ids only ever grow, so a row keeps its element as older lines fall off the top. */
export type OutputLine = { id: number } & LogEntry

export const OUTPUT_LINE_CAP = 500

export function appendOutputLines(
  lines: OutputLine[],
  added: OutputLine[],
): OutputLine[] {
  return [...lines, ...added].slice(-OUTPUT_LINE_CAP)
}

type OutputLineStyle = 'cmd' | 'out' | 'dim' | 'ok' | 'warn' | 'sep' | 'blank'

const STYLE_OF_KIND: Record<Exclude<LogKind, 'output'>, OutputLineStyle> = {
  command: 'cmd',
  marker: 'sep',
  interrupt: 'dim',
  note: 'out',
  warning: 'warn',
}

/** Cormux's own lines by their kind; a command's output by what common dev tools print. */
export function classifyOutputLine(entry: LogEntry): OutputLineStyle {
  if (entry.kind !== 'output') return STYLE_OF_KIND[entry.kind]
  const line = entry.text.trimEnd()
  if (!line.trim()) return 'blank'
  if (/port \d+ is in use/i.test(line)) return 'warn'
  if (
    /^Done in /i.test(line) ||
    /VITE v/i.test(line) ||
    /ready in \d/i.test(line)
  ) {
    return 'ok'
  }
  if (/\[vite\].*hmr update/i.test(line) || /reloading/i.test(line)) {
    return 'dim'
  }
  return 'out'
}

/** A marker is drawn as a rule with its words in the middle. */
export function outputLineText(entry: LogEntry): string {
  return entry.kind === 'marker' ? `──── ${entry.text} ────` : entry.text
}

export const OUTPUT_LINE_CLASS: Record<OutputLineStyle, string> = {
  cmd: 'text-foreground font-medium',
  out: 'text-foreground/90',
  dim: 'text-muted-foreground',
  ok: 'text-emerald-600 dark:text-emerald-400',
  warn: 'text-amber-600 dark:text-amber-400',
  sep: 'text-muted-foreground text-center',
  blank: '',
}
