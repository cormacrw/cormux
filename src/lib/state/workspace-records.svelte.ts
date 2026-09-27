import type { WorkspaceRecord } from '$lib/ipc/bindings'

export type AppRunStatus = 'stopped' | 'starting' | 'running'

export type WorkspaceRuntime = {
  appStatus: AppRunStatus
  port: number | null
  behind: number
}

export class WorkspaceRecordsStore {
  records = $state<WorkspaceRecord[]>([])
  runtimeById = $state<Record<string, WorkspaceRuntime>>({})

  hydrate(records: WorkspaceRecord[]) {
    this.records = records
    const next: Record<string, WorkspaceRuntime> = { ...this.runtimeById }
    for (const record of records) {
      if (!next[record.id]) {
        next[record.id] = { appStatus: 'stopped', port: null, behind: 0 }
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
      this.runtimeById[id] ?? { appStatus: 'stopped', port: null, behind: 0 }
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
  ) {
    const current = this.runtime(id)
    this.runtimeById = {
      ...this.runtimeById,
      [id]: { ...current, appStatus, port: port ?? current.port },
    }
  }
}

export const workspaceRecords = new WorkspaceRecordsStore()
