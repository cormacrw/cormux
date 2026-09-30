import { workspaceUi } from '$lib/state'

export function openDiffForPath(path: string) {
  workspaceUi.revealInChanges(path)
}
