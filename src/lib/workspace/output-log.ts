export type OutputLineStyle =
  'cmd' | 'out' | 'dim' | 'ok' | 'warn' | 'sep' | 'blank'

export function classifyOutputLine(raw: string): OutputLineStyle {
  const line = raw.trimEnd()
  if (!line.trim()) return 'blank'
  if (line.startsWith('> ') || line.startsWith('cmd:')) return 'cmd'
  if (line.includes('────')) return 'sep'
  if (
    /port \d+ is in use/i.test(line) ||
    /not available, using the next free port/i.test(line) ||
    /Address already in use/i.test(line)
  ) {
    return 'warn'
  }
  if (
    /^Done in /i.test(line) ||
    /VITE v/i.test(line) ||
    /ready in \d/i.test(line)
  ) {
    return 'ok'
  }
  if (
    line === '^C' ||
    /\[vite\].*hmr update/i.test(line) ||
    /reloading/i.test(line)
  ) {
    return 'dim'
  }
  return 'out'
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
