import { formatRelativeAge } from '$lib/homebase/relative-time'

/** Short label for inline conversation timestamps (`8m`, `2h`). */
export function formatThreadTime(fromMs: number, nowMs: number): string {
  const label = formatRelativeAge(fromMs, nowMs)
  if (label === 'just now') return 'now'
  return label.replace(' ago', '')
}

export function formatThreadTimeTitle(fromMs: number): string {
  return new Date(fromMs).toLocaleString()
}

export function seqToApproxMs(baseMs: number, seq: number): number {
  return baseMs + seq * 1000
}
