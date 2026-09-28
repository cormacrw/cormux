import type { WorkspaceAppRuntime, WorkspaceRecord } from '$lib/ipc/bindings'

export type AppRunStatus = 'stopped' | 'starting' | 'running' | 'crashed'

export type WorkspaceRuntime = {
  appStatus: AppRunStatus
  port: number | null
  behind: number
  exitCode: number | null
  logVersion: number
}

export class WorkspaceRecordsStore {
  records = $state<WorkspaceRecord[]>([])
  runtimeById = $state<Record<string, WorkspaceRuntime>>({})

  hydrate(records: WorkspaceRecord[]) {
    this.records = records
    const next: Record<string, WorkspaceRuntime> = { ...this.runtimeById }
    for (const record of records) {
      if (!next[record.id]) {
        next[record.id] = {
          appStatus: 'stopped',
          port: null,
          behind: 0,
          exitCode: null,
          logVersion: 0,
        }
      }
    }
    for (const id of Object.keys(next)) {
      if (!records.some((record) => record.id === id)) {
        delete next[id]
      }
    }
    this.runtimeById = next
  }

  getRecord(id: string) {
    return this.records.find((record) => record.id === id)
  }

  runtime(id: string): WorkspaceRuntime {
    return (
      this.runtimeById[id] ?? {
        appStatus: 'stopped',
        port: null,
        behind: 0,
        exitCode: null,
        logVersion: 0,
      }
    )
  }

  setBehind(id: string, behind: number) {
    const current = this.runtime(id)
    this.runtimeById = {
      ...this.runtimeById,
      [id]: { ...current, behind },
    }
  }

  setAppStatus(
    id: string,
    appStatus: AppRunStatus,
    port: number | null = null,
    exitCode: number | null = null,
  ) {
    const current = this.runtime(id)
    const nextPort =
      appStatus === 'stopped' || appStatus === 'crashed'
        ? null
        : (port ?? current.port)
    this.runtimeById = {
      ...this.runtimeById,
      [id]: {
        ...current,
        appStatus,
        port: nextPort,
        exitCode:
          appStatus === 'crashed' ? (exitCode ?? current.exitCode) : null,
      },
    }
  }

  applyApps(apps: WorkspaceAppRuntime[]) {
    if (apps.length === 0) return
    const next = { ...this.runtimeById }
    for (const app of apps) {
      const current = next[app.workspaceId] ?? this.runtime(app.workspaceId)
      next[app.workspaceId] = {
        ...current,
        appStatus: app.status as AppRunStatus,
        port: app.port,
        exitCode: app.exitCode,
      }
    }
    this.runtimeById = next
  }

  clearLog(id: string) {
    const current = this.runtime(id)
    this.runtimeById = {
      ...this.runtimeById,
      [id]: { ...current, logVersion: current.logVersion + 1 },
    }
  }
}

export const workspaceRecords = new WorkspaceRecordsStore()
