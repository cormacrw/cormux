import type { TimelineItem, TimelineRow } from './timeline-types'

export function buildTimelineRows(
  items: TimelineItem[],
  role: string,
  engine: string,
): TimelineRow[] {
  const rows: TimelineRow[] = []
  let prevSpeaker: 'user' | 'agent' | null = null

  for (const item of items) {
    if (item.kind === 'live') {
      rows.push({ kind: 'item', item })
      continue
    }
    const speaker = item.kind === 'user' ? 'user' : 'agent'
    if (speaker !== 'user' && speaker !== prevSpeaker) {
      rows.push({
        kind: 'speaker',
        id: `speaker-${item.id}`,
        speaker,
        role,
        engine,
        seq: item.seq,
        atMs: 'atMs' in item ? item.atMs : 0,
      })
      prevSpeaker = speaker
    }
    if (speaker === 'user') prevSpeaker = speaker
    rows.push({ kind: 'item', item })
  }

  return rows
}
