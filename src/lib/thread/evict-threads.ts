/**
 * Which threads to drop from memory: the least recently viewed beyond `limit`, never one on
 * screen. `recent` is oldest first.
 */
export function threadsToEvict(
  recent: string[],
  watched: ReadonlySet<string>,
  limit: number,
): string[] {
  const idle = recent.filter((id) => !watched.has(id))
  const room = Math.max(0, limit - (recent.length - idle.length))
  return idle.slice(0, Math.max(0, idle.length - room))
}
