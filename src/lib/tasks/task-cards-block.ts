/**
 * Work an agent has scoped before creating it, as a JSON tree inside
 * `<cormux-tasks>` tags: epics first, then the tasks under each epic, then
 * subtasks under each task. The optional `cormux-task-cards` skill (see
 * `skills/cormux-task-cards/` in the repo) asks agents to end a scoping reply
 * with this block; Cormux renders it as a hierarchy and hides the raw JSON.
 * Without the skill there is no block and replies render exactly as before.
 * A flat array of tasks (the skill's first format) still parses.
 */
const OPEN_TAG = '<cormux-tasks>'
const CLOSE_TAG = '</cormux-tasks>'

export type TaskDraftKind = 'epic' | 'task' | 'subtask'

/** In and out of an epic's scope. Either list may be missing. */
export type TaskDraftScope = { in: string[]; out: string[] }

/** One node in the scoped hierarchy, tracker-neutral for any creating skill. */
export type TaskDraft = {
  kind: TaskDraftKind
  title: string
  description?: string
  /** Epics: the outcome this body of work exists to produce. */
  outcome?: string
  /** Epics: what is and is not part of the work. */
  scope?: TaskDraftScope
  /** How anyone can tell this node is done. */
  acceptance: string[]
  priority?: string
  points?: number
  labels: string[]
  status?: string
  /** The tracker's own reference (e.g. `ENG-123`), once the task exists. */
  ref?: string
  url?: string
  children: TaskDraft[]
}

type RawDraft = Record<string, unknown>

function asString(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined
}

function asPoints(value: unknown): number | undefined {
  const points =
    typeof value === 'number'
      ? value
      : typeof value === 'string'
        ? Number(value)
        : NaN
  return Number.isFinite(points) && points > 0 ? points : undefined
}

function asStringList(value: unknown): string[] {
  if (typeof value === 'string') return value.trim() ? [value.trim()] : []
  if (!Array.isArray(value)) return []
  return value
    .map((item) => asString(item))
    .filter((item): item is string => item !== undefined)
}

function asKind(value: unknown, depth: number): TaskDraftKind {
  const kind = asString(value)
    ?.toLowerCase()
    .replace(/[\s_]+/g, '-')
  if (kind === 'epic') return 'epic'
  if (kind === 'subtask' || kind === 'sub-task' || kind === 'checklist')
    return 'subtask'
  if (kind === 'task' || kind === 'story' || kind === 'ticket') return 'task'
  // Unlabelled nodes take their level's natural kind: tasks at the top,
  // subtasks under a task.
  return depth >= 2 ? 'subtask' : 'task'
}

function childEntries(raw: RawDraft): unknown[] {
  for (const key of ['children', 'tasks', 'stories', 'subtasks']) {
    if (Array.isArray(raw[key])) return raw[key]
  }
  return []
}

function normalize(raw: RawDraft, depth: number): TaskDraft | null {
  const title = asString(raw.title) ?? asString(raw.name)
  if (!title) return null
  const kind = asKind(raw.kind ?? raw.type, depth)
  const draft: TaskDraft = {
    kind,
    title,
    labels: [],
    acceptance: [],
    children: [],
  }
  const description = asString(raw.description) ?? asString(raw.body)
  if (description) draft.description = description
  const outcome = asString(raw.outcome) ?? asString(raw.goal)
  if (outcome) draft.outcome = outcome
  const scope = raw.scope
  if (scope && typeof scope === 'object' && !Array.isArray(scope)) {
    const rawScope = scope as RawDraft
    const inScope = asStringList(rawScope.in ?? rawScope.includes)
    const outScope = asStringList(rawScope.out ?? rawScope.excludes)
    if (inScope.length || outScope.length)
      draft.scope = { in: inScope, out: outScope }
  }
  draft.acceptance = asStringList(
    raw.acceptance ?? raw.acceptanceCriteria ?? raw.doneWhen,
  )
  const priority = asString(raw.priority)
  if (priority) draft.priority = priority
  const points = asPoints(raw.points)
  if (points !== undefined) draft.points = points
  draft.labels = asStringList(raw.labels ?? raw.tags)
  const status = asString(raw.status)
  if (status) draft.status = status
  const ref = asString(raw.ref) ?? asString(raw.id)
  if (ref) draft.ref = ref
  const url = asString(raw.url)
  if (url) draft.url = url
  // The hierarchy is three levels deep; anything deeper is dropped rather
  // than rendered as an ever-narrower tree.
  if (depth < 3) {
    draft.children = childEntries(raw)
      .filter(
        (entry): entry is RawDraft =>
          Boolean(entry) && typeof entry === 'object' && !Array.isArray(entry),
      )
      .map((entry) => normalize(entry, depth + 1))
      .filter((child): child is TaskDraft => child !== null)
  }
  return draft
}

/** Agents often wrap JSON in a ```json fence even when asked not to. */
function stripCodeFence(body: string): string {
  if (!body.startsWith('```')) return body
  const rest = body.split('\n').slice(1).join('\n')
  return rest.replace(/```\s*$/, '').trim()
}

function topEntries(parsed: unknown): unknown[] | null {
  if (Array.isArray(parsed)) return parsed
  if (parsed && typeof parsed === 'object') {
    const raw = parsed as RawDraft
    for (const key of ['epics', 'tasks', 'drafts']) {
      if (Array.isArray(raw[key])) return raw[key]
    }
    return [parsed]
  }
  return null
}

function parseDrafts(body: string): TaskDraft[] | null {
  try {
    const entries = topEntries(JSON.parse(stripCodeFence(body.trim())))
    if (!entries) return null
    return entries
      .filter(
        (entry): entry is RawDraft =>
          Boolean(entry) && typeof entry === 'object' && !Array.isArray(entry),
      )
      .map((entry) => normalize(entry, 0))
      .filter((draft): draft is TaskDraft => draft !== null)
  } catch {
    return null
  }
}

export type DraftRollup = {
  epics: number
  tasks: number
  subtasks: number
  /** Points on work nodes; epics are never estimated. */
  points: number
  /** Leaf work (no children) carrying no estimate. */
  unestimatedLeaves: number
}

/** Counts across a hierarchy, for the summary line over the rendered tree. */
export function draftRollup(drafts: TaskDraft[]): DraftRollup {
  const rollup: DraftRollup = {
    epics: 0,
    tasks: 0,
    subtasks: 0,
    points: 0,
    unestimatedLeaves: 0,
  }
  const visit = (draft: TaskDraft) => {
    if (draft.kind === 'epic') rollup.epics += 1
    else if (draft.kind === 'task') rollup.tasks += 1
    else rollup.subtasks += 1
    if (draft.kind !== 'epic' && draft.points != null)
      rollup.points += draft.points
    if (
      draft.kind !== 'epic' &&
      draft.children.length === 0 &&
      draft.points == null
    )
      rollup.unestimatedLeaves += 1
    draft.children.forEach(visit)
  }
  drafts.forEach(visit)
  return rollup
}

/**
 * Splits a reply into the prose to show and any scoped work it carries.
 * A block still streaming (or its partial opening tag) is hidden with no
 * drafts yet. A complete but unparseable block stays visible in the prose,
 * so a malformed draft is never silently lost.
 */
export function splitTaskCards(text: string): {
  text: string
  drafts: TaskDraft[]
} {
  const start = text.lastIndexOf(OPEN_TAG)
  if (start < 0) {
    for (let length = OPEN_TAG.length - 1; length > 0; length--) {
      if (text.endsWith(OPEN_TAG.slice(0, length))) {
        return { text: text.slice(0, -length).trimEnd(), drafts: [] }
      }
    }
    return { text, drafts: [] }
  }
  const bodyStart = start + OPEN_TAG.length
  const close = text.indexOf(CLOSE_TAG, bodyStart)
  if (close < 0) return { text: text.slice(0, start).trimEnd(), drafts: [] }
  const drafts = parseDrafts(text.slice(bodyStart, close))
  if (drafts === null) return { text, drafts: [] }
  const after = text.slice(close + CLOSE_TAG.length)
  return { text: (text.slice(0, start) + after).trim(), drafts }
}
