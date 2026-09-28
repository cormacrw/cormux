const MINUTE_MS = 60_000
const HOUR_MS = 60 * MINUTE_MS

export function formatRelativeAge(fromMs: number, nowMs: number): string {
  const delta = Math.max(0, nowMs - fromMs)
  if (delta < MINUTE_MS) return 'just now'
  if (delta < HOUR_MS) {
    const minutes = Math.floor(delta / MINUTE_MS)
    return `${minutes}m ago`
  }
  const hours = Math.floor(delta / HOUR_MS)
  return `${hours}h ago`
}

export function parseTimestampMs(
  value: string | null | undefined,
): number | null {
  if (!value) return null
  const parsed = Date.parse(value)
  return Number.isNaN(parsed) ? null : parsed
}
