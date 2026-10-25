import type {
  ClickupBoard,
  ClickupTask,
  SprintPlanRow,
  TodoRow,
} from '$lib/ipc/bindings'
import { taskRef } from '$lib/clickup/board'
import { todoRef } from '$lib/todos/ref'
import { localDayOf, type Day } from './dates'

export type WeekFilter = 'all' | 'todo' | 'sprint'

/** A card on the Week page: a TODO or one of the user's ClickUp sprint tasks. */
export type WeekItem = {
  kind: 'todo' | 'sprint'
  /** The todo id or ClickUp task id. */
  id: string
  /** Unique across kinds, for keys and `data-week-card`. */
  key: string
  title: string
  /** The TODO's `CMUX-<n>`, or the task's ClickUp ID. */
  tag: string
  points: number | null
  plannedOn: Day | null
  done: boolean
  /** Finished in ClickUp, so the plan can't reopen it. */
  doneInClickup: boolean
  /** The local day it was completed here, if it was. */
  doneOn: Day | null
}

export function itemKey(kind: WeekItem['kind'], id: string) {
  return `${kind}:${id}`
}

function isDoneKind(kind: string) {
  return kind === 'done' || kind === 'closed'
}

function finishedInClickup(board: ClickupBoard, task: ClickupTask) {
  const status = board.statuses.find(
    (row) => row.name.toLowerCase() === task.status.toLowerCase(),
  )
  return status ? isDoneKind(status.kind) : false
}

export function todoItem(todo: TodoRow): WeekItem {
  return {
    kind: 'todo',
    id: todo.id,
    key: itemKey('todo', todo.id),
    title: todo.title,
    tag: todoRef(todo),
    points: null,
    plannedOn: todo.plannedOn,
    done: todo.doneAt != null,
    doneInClickup: false,
    doneOn: localDayOf(todo.doneAt),
  }
}

/** The user's sprint tasks with their local plan. TODOs come first, newest first, then the sprint. */
export function weekItems(
  todos: TodoRow[],
  board: ClickupBoard | null,
  plan: SprintPlanRow[],
): WeekItem[] {
  const rows = new Map(plan.map((row) => [row.taskId, row]))
  const sprint: WeekItem[] = (board?.tasks ?? []).map((task) => {
    const row = rows.get(task.id)
    const doneInClickup = board ? finishedInClickup(board, task) : false
    return {
      kind: 'sprint',
      id: task.id,
      key: itemKey('sprint', task.id),
      title: task.name,
      tag: taskRef(task),
      points: task.points,
      plannedOn: row?.plannedOn ?? null,
      done: doneInClickup || row?.doneAt != null,
      doneInClickup,
      doneOn: localDayOf(row?.doneAt),
    }
  })
  return [...todos.map(todoItem).reverse(), ...sprint]
}

/**
 * The loading area: open items with no day, plus unfinished ones planned for a day before
 * this week so nothing gets lost. Items completed today stay until tomorrow, so ticking one
 * off doesn't make it vanish. Sprint tasks already finished in ClickUp aren't open.
 */
export function loadingItems(
  items: WeekItem[],
  options: {
    today: Day
    /** The Monday of the real current week. */
    thisWeek: Day
    /** The days on screen; an item planned for one of them is in its lane instead. */
    shown: Day[]
    filter: WeekFilter
  },
): WeekItem[] {
  return items.filter((item) => {
    if (options.filter !== 'all' && item.kind !== options.filter) return false
    if (item.doneInClickup && item.plannedOn == null) return false
    const open = !item.done || item.doneOn === options.today
    if (item.plannedOn == null) return open
    return (
      !item.done &&
      item.plannedOn < options.thisWeek &&
      !options.shown.includes(item.plannedOn)
    )
  })
}

/** Each day's cards, in the order given. */
export function byDay(items: WeekItem[], days: Day[]): Map<Day, WeekItem[]> {
  const lanes = new Map<Day, WeekItem[]>(days.map((day) => [day, []]))
  for (const item of items) {
    if (item.plannedOn) lanes.get(item.plannedOn)?.push(item)
  }
  return lanes
}

/** `N of M planned done` for the days on screen. */
export function plannedDone(items: WeekItem[], days: Day[]) {
  const planned = items.filter(
    (item) => item.plannedOn != null && days.includes(item.plannedOn),
  )
  return {
    done: planned.filter((item) => item.done).length,
    total: planned.length,
  }
}
