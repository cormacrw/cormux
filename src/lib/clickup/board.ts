import type {
  ClickupBoard,
  ClickupStatus,
  ClickupTask,
} from '$lib/ipc/bindings'

export type SprintLane = { status: ClickupStatus; tasks: ClickupTask[] }

/** Settings keys for the picked workspace, space and sprint folder. */
export const CLICKUP_WORKSPACE_KEY = 'clickupWorkspaceId'
export const CLICKUP_SPACE_KEY = 'clickupSpaceId'
export const CLICKUP_FOLDER_KEY = 'clickupFolderId'
/** Lanes hidden on the board, as lower-cased status names in a JSON array. */
export const CLICKUP_HIDDEN_STATUSES_KEY = 'clickupHiddenStatuses'

/** The Fibonacci values most teams estimate in, offered as one-click choices. */
export const POINT_PRESETS = [0.5, 1, 2, 3, 5, 8] as const

function sameStatus(a: string, b: string) {
  return a.localeCompare(b, undefined, { sensitivity: 'accent' }) === 0
}

/** One lane per status, in ClickUp's order. Tasks keep the API's order within a lane. */
export function sprintLanes(
  statuses: ClickupStatus[],
  tasks: ClickupTask[],
): SprintLane[] {
  return statuses.map((status) => ({
    status,
    tasks: tasks.filter((task) => sameStatus(task.status, status.name)),
  }))
}

/** Anything malformed reads as nothing hidden. */
export function parseHiddenStatuses(raw: string | undefined): string[] {
  if (!raw) return []
  try {
    const parsed: unknown = JSON.parse(raw)
    return Array.isArray(parsed)
      ? parsed.filter((name): name is string => typeof name === 'string')
      : []
  } catch {
    return []
  }
}

export function statusKey(name: string) {
  return name.toLowerCase()
}

export function hasPoints(task: Pick<ClickupTask, 'points'>) {
  return task.points != null
}

export function formatPoints(points: number) {
  return Number.isInteger(points) ? String(points) : points.toFixed(1)
}

/** `in progress` → `In progress`; ClickUp stores status names in lower case. */
export function statusLabel(name: string) {
  return name.charAt(0).toUpperCase() + name.slice(1)
}

export function sprintTotals(tasks: ClickupTask[]) {
  let points = 0
  let unpointed = 0
  for (const task of tasks) {
    if (task.points == null) unpointed += 1
    else points += task.points
  }
  return { points, unpointed }
}

/** The board with only the tasks assigned to the API key's owner. */
export function assignedToMe(board: ClickupBoard): ClickupBoard {
  return {
    ...board,
    tasks: board.tasks.filter((task) =>
      task.assignees.some((user) => user.id === board.userId),
    ),
  }
}

/** ClickUp's `done` and `closed` status types both mean finished. */
function isDoneKind(kind: string) {
  return kind === 'done' || kind === 'closed'
}

/** Points finished out of all pointed tasks, and how many tasks are still open. */
export function sprintProgress(board: ClickupBoard | null) {
  let donePoints = 0
  let totalPoints = 0
  let openTasks = 0
  if (!board) return { donePoints, totalPoints, openTasks }
  const done = board.statuses
    .filter((status) => isDoneKind(status.kind))
    .map((status) => status.name)
  for (const task of board.tasks) {
    const finished = done.some((name) => sameStatus(name, task.status))
    if (!finished) openTasks += 1
    if (task.points == null) continue
    totalPoints += task.points
    if (finished) donePoints += task.points
  }
  return { donePoints, totalPoints, openTasks }
}

/** The ID people say out loud: the custom ID when the workspace has them, else ClickUp's own. */
export function taskRef(task: Pick<ClickupTask, 'id' | 'customId'>) {
  return task.customId ?? task.id
}

/** A copy of the board with one task in a new lane, for optimistic moves. */
export function withTaskStatus(
  board: ClickupBoard,
  taskId: string,
  status: ClickupStatus,
): ClickupBoard {
  return {
    ...board,
    tasks: board.tasks.map((task) =>
      task.id === taskId
        ? { ...task, status: status.name, statusColor: status.color }
        : task,
    ),
  }
}

export function withTaskPoints(
  board: ClickupBoard,
  taskId: string,
  points: number,
): ClickupBoard {
  return {
    ...board,
    tasks: board.tasks.map((task) =>
      task.id === taskId ? { ...task, points } : task,
    ),
  }
}

const DATE = new Intl.DateTimeFormat('en-US', {
  month: 'short',
  day: 'numeric',
})

/** `Sep 22 – Oct 5`, or nothing when the sprint has no dates. */
export function sprintRange(startMs: number | null, dueMs: number | null) {
  if (startMs == null || dueMs == null) return null
  return `${DATE.format(startMs)} – ${DATE.format(dueMs)}`
}

/** Whole days left in the sprint, counting today. */
export function daysLeft(dueMs: number | null, nowMs: number) {
  if (dueMs == null) return null
  return Math.max(0, Math.ceil((dueMs - nowMs) / 86_400_000))
}
