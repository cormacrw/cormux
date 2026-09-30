/** True when the UI missed at least one state event and must replace local state. */
export function hasVersionGap(lastSeen: number, incoming: number): boolean {
  return incoming > lastSeen + 1
}
