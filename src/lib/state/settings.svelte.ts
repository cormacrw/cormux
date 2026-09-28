import type { EngineKind, SettingRow } from '$lib/ipc/bindings'
import {
  readDefaultBase,
  readDefaultEngine,
} from '$lib/new-workspace/settings-defaults'

export class SettingsStore {
  reduceMotion = $state(false)
  autoApproveReadOnly = $state(true)
  defaultEngine = $state<EngineKind>('claude')
  defaultBase = $state('main')
  rows = $state<SettingRow[]>([])
  /** When set, Settings view scrolls/focuses this section (palette deep links). */
  focusSection = $state<string | null>(null)

  hydrate(
    settings: { reduceMotion?: boolean; autoApproveReadOnly?: boolean },
    rows: SettingRow[] = [],
  ) {
    if (settings.reduceMotion !== undefined) {
      this.reduceMotion = settings.reduceMotion
    }
    if (settings.autoApproveReadOnly !== undefined) {
      this.autoApproveReadOnly = settings.autoApproveReadOnly
    }
    this.rows = rows
    if (rows.length > 0) {
      this.defaultEngine = readDefaultEngine(rows)
      this.defaultBase = readDefaultBase(rows)
    }
  }

  toggleReduceMotion() {
    this.reduceMotion = !this.reduceMotion
  }
}

export const settings = new SettingsStore()
