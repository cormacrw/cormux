import { workspaceUi } from '$lib/state'

export function openDiffForPath(path: string) {
  workspaceUi.selectedDiffPath = path
  workspaceUi.changesOpen = true
}
