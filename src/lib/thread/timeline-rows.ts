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
  item: TimelineItem,
  role: string,
  engine: string,
): SpeakerRow {
  const known = speakerRows.get(item)
  if (known?.role === role && known.engine === engine) return known
  const row: SpeakerRow = {
    kind: 'speaker',
    id: `speaker-${item.id}`,
    role,
    engine,
    seq: item.seq,
    atMs: 'atMs' in item ? item.atMs : 0,
  }
  speakerRows.set(item, row)
  return row
}

/** The items, with a line naming the agent wherever it starts speaking. */
export function buildTimelineRows(
  items: TimelineItem[],
  role: string,
  engine: string,
): TimelineRow[] {
  const rows: TimelineRow[] = []
  let agentSpeaking = false

  for (const item of items) {
    // App events sit between turns, so they neither start nor break an agent's turn.
    if (item.kind === 'event') {
      rows.push(itemRow(item))
      continue
    }
    if (item.kind !== 'user' && !agentSpeaking) {
      rows.push(speakerRow(item, role, engine))
    }
    agentSpeaking = item.kind !== 'user'
    rows.push(itemRow(item))
  }

  return rows
}
