import type { SprintPlanRow } from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import { coreErrorText } from '$lib/feedback/core-error'
import { showToast } from '$lib/feedback/show-toast'
import {
  addDays,
  sqliteNow,
  today as currentDay,
  weekDays,
  weekStart,
  type Day,
} from '$lib/week/dates'
import type { WeekFilter, WeekItem } from '$lib/week/items'
import { clickup } from './clickup.svelte'

function failed(fallback: string, error: CoreError) {
  showToast({
    tone: 'bad',
    parts: [{ type: 'text', value: coreErrorText(error, fallback) }],
  })
}

export type WeekSelection = { kind: WeekItem['kind']; id: string }

/**
 * The Week page: which week is on screen, the loading area's filter, the open card, and the
 * local plan for ClickUp sprint tasks. Sprint plans never reach ClickUp.
 */
const HIDE_WEEKENDS_KEY = 'cormux-week-hide-weekends'

function readHideWeekends(): boolean {
  try {
    return globalThis.localStorage?.getItem(HIDE_WEEKENDS_KEY) === '1'
  } catch {
    return false
  }
}

export class WeekPlanStore {
  rows = $state<SprintPlanRow[]>([])
  today = $state<Day>(currentDay())
  /** The Monday of the week on screen. */
  start = $state<Day>(weekStart(currentDay()))
  filter = $state<WeekFilter>('all')
  /** The open TODO. An open sprint task lives in `clickup.selectedTaskId`, which its pane reads. */
  selectedTodoId = $state<string | null>(null)

  readonly days = $derived(weekDays(this.start))
  /** Saturday and Sunday out of the board; what's planned on them waits in the loading area. */
  hideWeekends = $state(readHideWeekends())
  /** The lanes on screen: Monday to Friday with weekends hidden, else all seven. */
  readonly visibleDays = $derived(
    this.hideWeekends ? this.days.slice(0, 5) : this.days,
  )
  readonly thisWeek = $derived(weekStart(this.today))
  readonly isThisWeek = $derived(this.start === this.thisWeek)

  hydrate(rows: SprintPlanRow[]) {
    this.rows = rows
  }

  /** Picks up a new day when the page opens after midnight. */
  refreshToday() {
    const today = currentDay()
    if (today === this.today) return
    const following = this.start === this.thisWeek
    this.today = today
    if (following) this.start = weekStart(today)
  }

  setHideWeekends(hide: boolean) {
    this.hideWeekends = hide
    try {
      globalThis.localStorage?.setItem(HIDE_WEEKENDS_KEY, hide ? '1' : '0')
    } catch {
      // Storage is optional; the choice still holds for this session.
    }
  }

  previousWeek() {
    this.start = addDays(this.start, -7)
  }

  nextWeek() {
    this.start = addDays(this.start, 7)
  }

  goToday() {
    this.refreshToday()
    this.start = this.thisWeek
  }

  rowFor(taskId: string): SprintPlanRow | undefined {
    return this.rows.find((row) => row.taskId === taskId)
  }

  selected(): WeekSelection | null {
    if (this.selectedTodoId) return { kind: 'todo', id: this.selectedTodoId }
    if (clickup.selectedTaskId)
      return { kind: 'sprint', id: clickup.selectedTaskId }
    return null
  }

  select(selection: WeekSelection | null) {
    this.selectedTodoId = selection?.kind === 'todo' ? selection.id : null
    const taskId = selection?.kind === 'sprint' ? selection.id : null
    if (clickup.selectedTaskId !== taskId) clickup.select(taskId)
  }

  private put(row: SprintPlanRow) {
    const rest = this.rows.filter((item) => item.taskId !== row.taskId)
    this.rows = [...rest, row]
  }

  private restore(taskId: string, before: SprintPlanRow | undefined) {
    if (before) this.put(before)
    else this.rows = this.rows.filter((item) => item.taskId !== taskId)
  }

  async planSprintTask(taskId: string, plannedOn: Day | null) {
    const before = this.rowFor(taskId)
    if ((before?.plannedOn ?? null) === plannedOn) return
    this.put({ taskId, plannedOn, doneAt: before?.doneAt ?? null })
    const result = await commands.planSprintTask(taskId, plannedOn)
    if (result.status === 'error') {
      this.restore(taskId, before)
      failed('Could not plan the task', result.error)
      return
    }
    this.put(result.data)
  }

  async setSprintTaskDone(taskId: string, done: boolean) {
    const before = this.rowFor(taskId)
    if ((before?.doneAt != null) === done) return
    this.put({
      taskId,
      plannedOn: before?.plannedOn ?? null,
      doneAt: done ? sqliteNow() : null,
    })
    const result = await commands.setSprintTaskDone(taskId, done)
    if (result.status === 'error') {
      this.restore(taskId, before)
      failed('Could not update the task', result.error)
      return
    }
    this.put(result.data)
  }
}

export const weekPlan = new WeekPlanStore()
