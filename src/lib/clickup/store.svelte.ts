import type {
  ClickupBoard,
  ClickupTaskDetail,
  Settings,
} from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import { onSnapshot } from '$lib/core/sync'
import { coalesce } from '$lib/core/coalesce'
import { coreErrorText } from '$lib/core/core-error'
import { deepEqual } from '$lib/deep-equal'
import { settings } from '$lib/settings/store.svelte'
import {
  assignedToMe,
  sameStatus,
  sprintLanes,
  sprintProgress,
  statusKey,
  statusLabel,
  withTaskPoints,
  withTaskStatus,
} from './board'

import type { ActionFailure } from '$lib/feedback/action-failure'

/**
 * How long after the last write the board reloads, so it picks up what ClickUp did in
 * response (automations that assign or move tasks) without racing the write itself.
 */
const SETTLE_MS = 800

/**
 * The current ClickUp sprint. Every ClickUp surface hides unless an API key is stored;
 * the board also needs a sprint folder picked in Settings.
 */
export class ClickupStore {
  configured = $state(false)
  folderId = $state('')
  board = $state<ClickupBoard | null>(null)
  loading = $state(false)
  error = $state<string | null>(null)
  selectedTaskId = $state<string | null>(null)
  details = $state<Record<string, ClickupTaskDetail>>({})
  detailError = $state<string | null>(null)
  hiddenStatuses = $state<string[]>([])

  /** Bumped by every write so a refresh that started earlier can't undo it. */
  private writes = 0
  /** Writes still waiting on ClickUp; a refresh that lands meanwhile is stale. */
  private inFlight = 0
  private settleTimer: ReturnType<typeof setTimeout> | undefined

  readonly ready = $derived(this.configured && this.folderId !== '')
  /** The board narrowed to the current user's tasks; every surface reads this one. */
  readonly mine = $derived(this.board && assignedToMe(this.board))
  /** Every lane, hidden ones included, for the lanes menu. */
  readonly allLanes = $derived(
    this.mine ? sprintLanes(this.mine.statuses, this.mine.tasks) : [],
  )
  readonly lanes = $derived(
    this.allLanes.filter((lane) => !this.isHidden(lane.status.name)),
  )
  readonly hiddenLaneCount = $derived(this.allLanes.length - this.lanes.length)
  readonly progress = $derived(sprintProgress(this.mine))
  readonly selectedTask = $derived(
    this.mine?.tasks.find((task) => task.id === this.selectedTaskId) ?? null,
  )

  hydrate(configured: boolean, chosen: Settings) {
    const folderId = chosen.clickupFolderId
    const hidden = chosen.clickupHiddenStatuses
    // Only a real change: a new array here would rebuild every lane on every snapshot.
    if (!deepEqual(hidden, this.hiddenStatuses)) this.hiddenStatuses = hidden
    const changed = configured !== this.configured || folderId !== this.folderId
    this.configured = configured
    this.folderId = folderId
    if (!changed) return
    this.board = null
    this.details = {}
    this.selectedTaskId = null
    this.error = null
    if (this.ready) void this.refresh()
  }

  async refresh() {
    if (!this.ready) return
    this.loading = true
    try {
      await this.load()
    } finally {
      this.loading = false
    }
  }

  private load = coalesce(async () => {
    const writes = this.writes
    const folderId = this.folderId
    const result = await commands.clickupBoard()
    if (
      writes !== this.writes ||
      this.inFlight > 0 ||
      folderId !== this.folderId
    ) {
      return
    }
    if (result.status === 'error') {
      this.error = coreErrorText(result.error, 'Could not load the sprint')
      return
    }
    this.error = null
    this.board = result.data
  })

  private async write<T>(run: () => Promise<T>): Promise<T> {
    this.writes += 1
    this.inFlight += 1
    try {
      return await run()
    } finally {
      this.inFlight -= 1
      clearTimeout(this.settleTimer)
      this.settleTimer = setTimeout(() => void this.refresh(), SETTLE_MS)
    }
  }

  isHidden(statusName: string) {
    return this.hiddenStatuses.includes(statusKey(statusName))
  }

  setLaneHidden(statusName: string, hidden: boolean) {
    const key = statusKey(statusName)
    const rest = this.hiddenStatuses.filter((name) => name !== key)
    return this.persistHidden(hidden ? [...rest, key] : rest)
  }

  showAllLanes() {
    return this.persistHidden([])
  }

  private persistHidden(next: string[]): Promise<CoreError | null> {
    this.hiddenStatuses = next
    return settings.update({ clickupHiddenStatuses: next })
  }

  select(taskId: string | null) {
    this.selectedTaskId = taskId
    if (taskId) void this.loadDetail(taskId)
  }

  async loadDetail(taskId: string) {
    this.detailError = null
    const result = await commands.clickupTask(taskId)
    if (result.status === 'error') {
      if (this.selectedTaskId === taskId) {
        this.detailError = coreErrorText(
          result.error,
          'Could not load the task',
        )
      }
      return
    }
    this.details = { ...this.details, [taskId]: result.data }
  }

  private patchDetail(
    taskId: string,
    patch: Partial<ClickupTaskDetail['task']>,
  ) {
    const detail = this.details[taskId]
    if (!detail) return
    this.details = {
      ...this.details,
      [taskId]: { ...detail, task: { ...detail.task, ...patch } },
    }
  }

  async moveTask(
    taskId: string,
    statusName: string,
  ): Promise<ActionFailure | null> {
    const board = this.board
    const status = board?.statuses.find((row) => row.name === statusName)
    const task = board?.tasks.find((row) => row.id === taskId)
    if (!board || !status || !task || sameStatus(task.status, status.name))
      return null
    this.board = withTaskStatus(board, taskId, status)
    this.patchDetail(taskId, { status: status.name, statusColor: status.color })
    const result = await this.write(() =>
      commands.setClickupTaskStatus(taskId, status.name),
    )
    if (result.status === 'error') {
      this.board =
        this.board &&
        withTaskStatus(this.board, taskId, {
          ...status,
          name: task.status,
          color: task.statusColor,
        })
      this.patchDetail(taskId, {
        status: task.status,
        statusColor: task.statusColor,
      })
      return {
        error: result.error,
        doing: `Could not move the task to ${statusLabel(status.name)}`,
      }
    }
    return null
  }

  async setPoints(
    taskId: string,
    points: number,
  ): Promise<ActionFailure | null> {
    const board = this.board
    const task = board?.tasks.find((row) => row.id === taskId)
    if (!board || !task || task.points === points) return null
    this.board = withTaskPoints(board, taskId, points)
    this.patchDetail(taskId, { points })
    const result = await this.write(() =>
      commands.setClickupTaskPoints(taskId, points),
    )
    if (result.status === 'error') {
      this.board = this.board && withTaskPoints(this.board, taskId, task.points)
      this.patchDetail(taskId, { points: task.points })
      return { error: result.error, doing: 'Could not set sprint points' }
    }
    return null
  }
}

export const clickup = new ClickupStore()

onSnapshot((snapshot) =>
  clickup.hydrate(snapshot.clickupConfigured, snapshot.settings),
)
