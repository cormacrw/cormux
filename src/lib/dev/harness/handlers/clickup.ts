import {
  fixtureClickupBoard,
  fixtureClickupDetail,
} from '../../fixture-clickup'
import { fixtureSnapshot } from '../../fixture-snapshot'
import type { HarnessRuntime } from '../runtime'
import type { HandlerSlice } from '../types'

export function clickupHandlers({
  controls,
  emitStateChanged,
}: HarnessRuntime) {
  const board = fixtureClickupBoard()
  const findTask = (taskId: string) =>
    board.tasks.find((row) => row.id === taskId)

  return {
    set_clickup_api_key: ({ key }: { key: string }) => {
      if (!key.startsWith('pk_')) {
        throw { kind: 'Clickup', message: 'ClickUp rejected the API key' }
      }
      fixtureSnapshot.clickupConfigured = true
      emitStateChanged()
      return null
    },
    clear_clickup_api_key: () => {
      fixtureSnapshot.clickupConfigured = false
      emitStateChanged()
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
      return fixtureClickupDetail(structuredClone(found))
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
