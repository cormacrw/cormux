import { toastCoreError } from '$lib/feedback/wire-feedback'
import { clickup } from './store.svelte'
import { writeErrorText, type ClickupWriteError } from './write-error'

function toastFailure(failure: ClickupWriteError | null) {
  if (failure) toastCoreError(writeErrorText(failure))
}

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
