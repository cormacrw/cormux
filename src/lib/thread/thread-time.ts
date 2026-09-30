/** `YYYY-MM-DD HH:MM:SS` from SQLite is UTC. */
export function eventCreatedAtMs(createdAt: string): number {
  if (!createdAt) return 0
  const ms = Date.parse(createdAt.replace(' ', 'T') + 'Z')
  return Number.isNaN(ms) ? 0 : ms
}

/** Clock time to the minute (`11:04 PM`). Empty when the event has no timestamp. */
export function formatThreadTime(fromMs: number): string | null {
  if (!fromMs) return null
  return new Date(fromMs).toLocaleTimeString([], {
    hour: 'numeric',
    minute: '2-digit',
  })
}

export function formatThreadTimeTitle(fromMs: number): string {
  return new Date(fromMs).toLocaleString()
}
