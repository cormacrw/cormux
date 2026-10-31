import { workspaceUi } from '$lib/state'

export function openDiffForPath(path: string, line: number | null = null) {
  workspaceUi.revealInChanges(path, line)
}
