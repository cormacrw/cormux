import { clickupTaskDetail } from '../clickup-fixtures'
import { CHANGE, type HarnessRuntime } from '../runtime'
import type { HandlerSlice } from '../types'

export function clickupHandlers({
  db,
  controls,
  emitStateChanged,
}: HarnessRuntime) {
  const board = db.clickup
  const findTask = (taskId: string) =>
    board.tasks.find((row) => row.id === taskId)

  return {
    set_clickup_api_key: ({ key }: { key: string }) => {
      if (!key.startsWith('pk_')) {
        throw { kind: 'Clickup', message: 'ClickUp rejected the API key' }
      }
      db.snapshot.clickupConfigured = true
      emitStateChanged(CHANGE.state)
      return null
    },
    clear_clickup_api_key: () => {
      db.snapshot.clickupConfigured = false
      emitStateChanged(CHANGE.state)
      return null
    },
    clickup_workspaces: () => [{ id: 'team-1', name: 'Acme' }],
    clickup_spaces: () => [{ id: 'space-1', name: 'Engineering' }],
    clickup_folders: () => [
      { id: 'folder-sprints', name: 'Sprints' },
      { id: 'folder-roadmap', name: 'Roadmap' },
    ],
    clickup_board: () => structuredClone(board),
    clickup_task: ({ taskId }: { taskId: string }) => {
      const found = findTask(taskId)
      if (!found) throw { kind: 'Clickup', message: 'ClickUp: Task not found' }
      return clickupTaskDetail(structuredClone(found))
    },
    set_clickup_task_status: (args: { taskId: string; status: string }) => {
      controls.clickupCalls.push({ cmd: 'set_clickup_task_status', args })
      const found = findTask(args.taskId)
      if (found) {
        const status = board.statuses.find((row) => row.name === args.status)
        found.status = args.status
        found.statusColor = status?.color ?? null
      }
      return null
    },
    set_clickup_task_points: (args: { taskId: string; points: number }) => {
      controls.clickupCalls.push({ cmd: 'set_clickup_task_points', args })
      const found = findTask(args.taskId)
      if (found) found.points = args.points
      return null
    },
  } satisfies HandlerSlice
}
