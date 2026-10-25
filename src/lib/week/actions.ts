import type { Day } from './dates'
import type { WeekItem } from './items'
import { todos } from '$lib/state/todos.svelte'
import { weekPlan } from '$lib/state/week-plan.svelte'

/** Plans a TODO or sprint task for a day, or takes it off the plan. */
export function planItem(item: WeekItem, day: Day | null) {
  if (item.kind === 'todo') void todos.plan(item.id, day)
  else void weekPlan.planSprintTask(item.id, day)
}

/** Ticks a card off or back on; a task finished in ClickUp stays done. */
export function toggleItemDone(item: WeekItem) {
  if (item.doneInClickup) return
  if (item.kind === 'todo') void todos.setDone(item.id, !item.done)
  else void weekPlan.setSprintTaskDone(item.id, !item.done)
}
