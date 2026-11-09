import { defineBlock, tag } from '../block'

/**
 * Work an agent has scoped before creating it, as a JSON array inside
 * `<cormux-tasks>` tags: epics, the tasks under each, then subtasks under each
 * task. The format is the one `skills/cormux-task-cards/SKILL.md` asks agents
 * to write, and only that; Cormux renders it as a hierarchy and hides the JSON.
 */

type TaskDraftKind = 'epic' | 'task' | 'subtask'

/** In and out of an epic's scope. Either list may be missing. */
type TaskDraftScope = { in: string[]; out: string[] }

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
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? value
    : undefined
}

function asStringList(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return value
    .map((item) => asString(item))
    .filter((item): item is string => item !== undefined)
}

function asKind(value: unknown, depth: number): TaskDraftKind {
  if (value === 'epic' || value === 'task' || value === 'subtask') return value
  // Unlabelled nodes take their level's kind: tasks at the top and under an
  // epic, subtasks under a task.
  return depth >= 2 ? 'subtask' : 'task'
}

function normalize(raw: RawDraft, depth: number): TaskDraft | null {
  const title = asString(raw.title)
  if (!title) return null
  const kind = asKind(raw.kind, depth)
  const draft: TaskDraft = {
    kind,
    title,
    labels: [],
    acceptance: [],
    children: [],
  }
  const description = asString(raw.description)
  if (description) draft.description = description
  const outcome = asString(raw.outcome)
  if (outcome) draft.outcome = outcome
  const scope = raw.scope
  if (scope && typeof scope === 'object' && !Array.isArray(scope)) {
    const rawScope = scope as RawDraft
    const inScope = asStringList(rawScope.in)
    const outScope = asStringList(rawScope.out)
    if (inScope.length || outScope.length)
      draft.scope = { in: inScope, out: outScope }
  }
  draft.acceptance = asStringList(raw.acceptance)
  const priority = asString(raw.priority)
  if (priority) draft.priority = priority
  const points = asPoints(raw.points)
  if (points !== undefined) draft.points = points
  draft.labels = asStringList(raw.labels)
  const status = asString(raw.status)
  if (status) draft.status = status
  const ref = asString(raw.ref)
  if (ref) draft.ref = ref
  const url = asString(raw.url)
  if (url) draft.url = url
  // The hierarchy is three levels deep; anything deeper is dropped rather
  // than rendered as an ever-narrower tree.
  if (depth < 3 && Array.isArray(raw.children)) {
    draft.children = raw.children
      .filter(
        (entry): entry is RawDraft =>
          Boolean(entry) && typeof entry === 'object' && !Array.isArray(entry),
      )
      .map((entry) => normalize(entry, depth + 1))
      .filter((child): child is TaskDraft => child !== null)
  }
  return draft
}

function parseDrafts(body: string): TaskDraft[] | null {
  try {
    const entries: unknown = JSON.parse(body)
    if (!Array.isArray(entries)) return null
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

type DraftRollup = {
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

function draftText(draft: TaskDraft): string[] {
  return [
    draft.title,
    draft.description ?? '',
    draft.outcome ?? '',
    ...(draft.scope?.in ?? []),
    ...(draft.scope?.out ?? []),
    ...draft.acceptance,
    ...draft.labels,
    ...draft.children.flatMap(draftText),
  ]
}

/** A complete block that doesn't parse stays in the prose, so a malformed draft is never lost. */
export const tasksBlock = defineBlock({
  kind: 'tasks',
  match: tag('cormux-tasks'),
  parse: parseDrafts,
  text: (drafts) => drafts.flatMap(draftText).filter(Boolean).join('\n'),
})
