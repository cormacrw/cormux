import { toastFailure } from '$lib/feedback/action-failure'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { clickup } from './store.svelte'

export async function moveTask(taskId: string, statusName: string) {
  toastFailure(await clickup.moveTask(taskId, statusName))
}

export async function setTaskPoints(taskId: string, points: number) {
  toastFailure(await clickup.setPoints(taskId, points))
}

export async function setLaneHidden(statusName: string, hidden: boolean) {
  const error = await clickup.setLaneHidden(statusName, hidden)
  if (error) toastCoreError(error)
}

export async function showAllLanes() {
  const error = await clickup.showAllLanes()
  if (error) toastCoreError(error)
}
