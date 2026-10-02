import type {
  ClickupBoard,
  ClickupTaskDetail,
  SettingRow,
} from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import { coreErrorText } from '$lib/feedback/core-error'
import { showToast } from '$lib/feedback/show-toast'
import {
  CLICKUP_FOLDER_KEY,
  inProgressTasks,
  sprintLanes,
  statusLabel,
  withTaskPoints,
  withTaskStatus,
} from '$lib/clickup/board'

function failed(fallback: string, error: CoreError) {
  showToast({
    tone: 'bad',
    parts: [{ type: 'text', value: coreErrorText(error, fallback) }],
  })
}

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

  /** Bumped by every write so a refresh that started earlier can't undo it. */
  private writes = 0
  /** Writes still waiting on ClickUp; a refresh that lands meanwhile is stale. */
  private inFlight = 0
  /** A refresh asked for while one was running, run once that one finishes. */
  private refreshQueued = false
  private settleTimer: ReturnType<typeof setTimeout> | undefined

  readonly ready = $derived(this.configured && this.folderId !== '')
  readonly lanes = $derived(
    this.board ? sprintLanes(this.board.statuses, this.board.tasks) : [],
  )
  readonly inProgress = $derived(inProgressTasks(this.board))
  readonly selectedTask = $derived(
    this.board?.tasks.find((task) => task.id === this.selectedTaskId) ?? null,
  )

  hydrate(configured: boolean, rows: SettingRow[]) {
    const folderId =
      rows.find((row) => row.key === CLICKUP_FOLDER_KEY)?.value ?? ''
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
    if (this.loading) {
      this.refreshQueued = true
      return
    }
    this.loading = true
    const writes = this.writes
    const folderId = this.folderId
    const result = await commands.clickupBoard()
    this.loading = false
    if (this.refreshQueued) {
      this.refreshQueued = false
      void this.refresh()
      return
    }
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
  }

  /**
   * Reloads shortly after the last write settles, so the board picks up what ClickUp did
   * in response (automations that assign or move tasks) without racing the write itself.
   */
  private async write<T>(run: () => Promise<T>): Promise<T> {
    this.writes += 1
    this.inFlight += 1
    try {
      return await run()
    } finally {
      this.inFlight -= 1
      clearTimeout(this.settleTimer)
      this.settleTimer = setTimeout(() => void this.refresh(), 800)
    }
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

  async moveTask(taskId: string, statusName: string) {
    const board = this.board
    const status = board?.statuses.find((row) => row.name === statusName)
    const task = board?.tasks.find((row) => row.id === taskId)
    if (!board || !status || !task || task.status === status.name) return
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
      failed(
        `Could not move the task to ${statusLabel(status.name)}`,
        result.error,
      )
    }
  }

  async setPoints(taskId: string, points: number) {
    const board = this.board
    const task = board?.tasks.find((row) => row.id === taskId)
    if (!board || !task || task.points === points) return
    this.board = withTaskPoints(board, taskId, points)
    this.patchDetail(taskId, { points })
    const result = await this.write(() =>
      commands.setClickupTaskPoints(taskId, points),
    )
    if (result.status === 'error') {
      this.board = this.board && {
        ...this.board,
        tasks: this.board.tasks.map((row) =>
          row.id === taskId ? { ...row, points: task.points } : row,
        ),
      }
      this.patchDetail(taskId, { points: task.points })
      failed('Could not set sprint points', result.error)
    }
  }
}

export const clickup = new ClickupStore()
