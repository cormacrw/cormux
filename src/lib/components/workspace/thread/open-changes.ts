import { workspaceUi } from '$lib/state'

export function openDiffForPath(path: string) {
  workspaceUi.revealDiffPath = path
  workspaceUi.openTab('changes')
}
