// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type {
  GitConflictState,
  WorkspaceAppRuntime,
  WorkspaceAppStatus,
  WorkspaceGitRuntime,
  WorkspaceRecord,
} from '$lib/ipc/bindings'
import { deepEqual } from '$lib/deep-equal'
import { reuseRows } from './reuse-rows'

type WorkspaceRuntime = {
  appStatus: WorkspaceAppStatus
  port: number | null
  /** The app announced an `https://` URL. */
  https: boolean
  behind: number
  ahead: number
  conflict: GitConflictState | null
  exitCode: number | null
  logVersion: number
}

const IDLE_RUNTIME: WorkspaceRuntime = Object.freeze({
  appStatus: 'stopped',
  port: null,
  https: false,
  behind: 0,
  ahead: 0,
  conflict: null,
  exitCode: null,
  logVersion: 0,
})

export class WorkspaceRecordsStore {
  // Raw and reused: a snapshot that changed nothing here changes nothing downstream.
  records = $state.raw<WorkspaceRecord[]>([])
  runtimeById = $state.raw<Record<string, WorkspaceRuntime>>({})

  private readonly byId = $derived(
    new Map(this.records.map((record) => [record.id, record])),
  )

  hydrate(records: WorkspaceRecord[]) {
    this.records = reuseRows(this.records, records)
    const alive = new Set(records.map((record) => record.id))
    const current = this.runtimeById
    const stale = Object.keys(current).some((id) => !alive.has(id))
    const missing = records.some((record) => !current[record.id])
    if (!stale && !missing) return
    const next: Record<string, WorkspaceRuntime> = {}
    for (const record of records) {
      next[record.id] = current[record.id] ?? IDLE_RUNTIME
    }
    this.runtimeById = next
  }

  getRecord(id: string) {
    return this.byId.get(id)
  }

  runtime(id: string): WorkspaceRuntime {
    return this.runtimeById[id] ?? IDLE_RUNTIME
  }

  /** Replaces entries that changed, keeping the rest (and the map) as they were. */
  private patchRuntimes(patches: Record<string, Partial<WorkspaceRuntime>>) {
    let next: Record<string, WorkspaceRuntime> | null = null
    for (const [id, patch] of Object.entries(patches)) {
      const current = this.runtime(id)
      const merged = { ...current, ...patch }
      if (deepEqual(current, merged)) continue
      next ??= { ...this.runtimeById }
      next[id] = merged
    }
    if (next) this.runtimeById = next
  }

  applyGitStats(rows: WorkspaceGitRuntime[]) {
    if (rows.length === 0) return
    const patches: Record<string, Partial<WorkspaceRuntime>> = {}
    for (const row of rows) {
      patches[row.workspaceId] = {
        behind: row.behind,
        ahead: row.ahead,
        conflict: row.conflict,
      }
    }
    this.patchRuntimes(patches)
  }

  setAppStatus(
    id: string,
    appStatus: WorkspaceAppStatus,
    port: number | null = null,
    exitCode: number | null = null,
  ) {
    const current = this.runtime(id)
    const nextPort =
      appStatus === 'stopped' || appStatus === 'crashed'
        ? null
        : (port ?? current.port)
    this.patchRuntimes({
      [id]: {
        appStatus,
        port: nextPort,
        https: nextPort === null ? false : current.https,
        exitCode:
          appStatus === 'crashed' ? (exitCode ?? current.exitCode) : null,
      },
    })
  }

  applyApps(apps: WorkspaceAppRuntime[]) {
    if (apps.length === 0) return
    const patches: Record<string, Partial<WorkspaceRuntime>> = {}
    for (const app of apps) {
      patches[app.workspaceId] = {
        appStatus: app.status,
        port: app.port,
        https: app.https,
        exitCode: app.exitCode,
      }
    }
    this.patchRuntimes(patches)
  }

  clearLog(id: string) {
    this.patchRuntimes({
      [id]: { logVersion: this.runtime(id).logVersion + 1 },
    })
  }
}

export const workspaceRecords = new WorkspaceRecordsStore()
