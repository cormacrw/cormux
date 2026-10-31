import type { TimelineItem, TimelineRow } from './timeline-types'

type ItemRow = Extract<TimelineRow, { kind: 'item' }>
type SpeakerRow = Extract<TimelineRow, { kind: 'speaker' }>

// An unchanged item keeps its row object, so rebuilding the rows doesn't re-run that row's template.
const itemRows = new WeakMap<TimelineItem, ItemRow>()
const speakerRows = new WeakMap<TimelineItem, SpeakerRow>()

function itemRow(item: TimelineItem): ItemRow {
  let row = itemRows.get(item)
  if (!row) {
    row = { kind: 'item', item }
    itemRows.set(item, row)
  }
  return row
}

function speakerRow(
  item: Exclude<TimelineItem, { kind: 'live' | 'event' }>,
  speaker: 'agent',
  role: string,
  engine: string,
): SpeakerRow {
  const known = speakerRows.get(item)
  if (known?.role === role && known.engine === engine) return known
  const row: SpeakerRow = {
    kind: 'speaker',
    id: `speaker-${item.id}`,
    speaker,
    role,
    engine,
    seq: item.seq,
    atMs: 'atMs' in item ? item.atMs : 0,
  }
  speakerRows.set(item, row)
  return row
}

export function buildTimelineRows(
  items: TimelineItem[],
  role: string,
  engine: string,
): TimelineRow[] {
  const rows: TimelineRow[] = []
  let prevSpeaker: 'user' | 'agent' | null = null

  for (const item of items) {
    // App events sit between turns, so they neither start nor break an agent's turn.
    if (item.kind === 'live' || item.kind === 'event') {
      rows.push(itemRow(item))
      continue
    }
    if (item.kind !== 'user' && prevSpeaker !== 'agent') {
      rows.push(speakerRow(item, 'agent', role, engine))
      prevSpeaker = 'agent'
    }
    if (item.kind === 'user') prevSpeaker = 'user'
    rows.push(itemRow(item))
  }

  return rows
}
