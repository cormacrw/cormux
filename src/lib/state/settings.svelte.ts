import type { EngineKind, SettingRow } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import {
  DEFAULT_WORKTREE_ROOT,
  readBooleanSetting,
  readDefaultBase,
  readDefaultEngine,
  readStringSetting,
} from '$lib/new-workspace/settings-defaults'
import {
  parseScratchMacros,
  SCRATCH_MACROS_KEY,
  serializeScratchMacros,
  type ScratchMacro,
} from '$lib/settings/scratch-macros'

function upsertRow(rows: SettingRow[], key: string, value: string) {
  const next = rows.filter((row) => row.key !== key)
  next.push({ key, value })
  return next
}

export class SettingsStore {
  reduceMotion = $state(false)
  autoApproveReadOnly = $state(true)
  runEverything = $state(false)
  defaultEngine = $state<EngineKind>('claude')
  defaultBase = $state('main')
  /** Preferred repo id; resolve with `resolveDefaultRepoId` since it may be removed. */
  defaultRepo = $state('')
  teardownAfterMerge = $state(true)
  worktreeRoot = $state(DEFAULT_WORKTREE_ROOT)
  notifyApprovals = $state(true)
  notifyReviewFinished = $state(true)
  scratchMacros = $state<ScratchMacro[]>([])
  rows = $state<SettingRow[]>([])
  /** When set, Settings view scrolls/focuses this section (palette deep links). */
  focusSection = $state<string | null>(null)
  /** Expand this repo and focus its run command field (Output empty state). */
  focusRepoRunCommand = $state<string | null>(null)
  /** Expand this repo's config panel (add repo, deep links). */
  expandRepoId = $state<string | null>(null)
  /** Focus Worktree setup after expanding (add repo). */
  focusRepoSetup = $state<string | null>(null)

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
      this.defaultRepo = readStringSetting(rows, 'defaultRepo', '')
      this.teardownAfterMerge = readBooleanSetting(
        rows,
        'teardownAfterMerge',
        true,
      )
      this.worktreeRoot = readStringSetting(
        rows,
        'worktreeRoot',
        DEFAULT_WORKTREE_ROOT,
      )
      this.notifyApprovals = readBooleanSetting(rows, 'notifyApprovals', true)
      this.notifyReviewFinished = readBooleanSetting(
        rows,
        'notifyReviewFinished',
        true,
      )
      this.runEverything = readBooleanSetting(rows, 'runEverything', false)
      this.scratchMacros = parseScratchMacros(
        rows.find((row) => row.key === SCRATCH_MACROS_KEY)?.value,
      )
    }
  }

  private applyLocal(key: string, value: string) {
    this.rows = upsertRow(this.rows, key, value)
  }

  async persist(key: string, value: string) {
    this.applyLocal(key, value)
    const result = await commands.setSetting({ key, value })
    if (result.status === 'error') {
      toastCoreError(result.error)
    }
  }

  async setDefaultEngine(engine: EngineKind) {
    this.defaultEngine = engine
    await this.persist('defaultEngine', engine)
  }

  async setDefaultBase(branch: string) {
    const trimmed = branch.trim()
    this.defaultBase = trimmed || 'main'
    await this.persist('defaultBase', this.defaultBase)
  }

  async setDefaultRepo(repoId: string) {
    this.defaultRepo = repoId
    await this.persist('defaultRepo', repoId)
  }

  async setReduceMotion(next: boolean) {
    this.reduceMotion = next
    await this.persist('reduceMotion', next ? 'true' : 'false')
  }

  toggleReduceMotion() {
    void this.setReduceMotion(!this.reduceMotion)
  }

  async setAutoApproveReadOnly(next: boolean) {
    this.autoApproveReadOnly = next
    await this.persist('autoApproveReadOnly', next ? 'true' : 'false')
  }

  async setRunEverything(next: boolean) {
    this.runEverything = next
    await this.persist('runEverything', next ? 'true' : 'false')
  }

  async setTeardownAfterMerge(next: boolean) {
    this.teardownAfterMerge = next
    await this.persist('teardownAfterMerge', next ? 'true' : 'false')
  }

  async setWorktreeRoot(path: string) {
    const trimmed = path.trim() || DEFAULT_WORKTREE_ROOT
    this.worktreeRoot = trimmed
    await this.persist('worktreeRoot', trimmed)
  }

  async setNotifyApprovals(next: boolean) {
    this.notifyApprovals = next
    await this.persist('notifyApprovals', next ? 'true' : 'false')
  }

  async setScratchMacros(next: ScratchMacro[]) {
    this.scratchMacros = next
    await this.persist(SCRATCH_MACROS_KEY, serializeScratchMacros(next))
  }

  async setNotifyReviewFinished(next: boolean) {
    this.notifyReviewFinished = next
    await this.persist('notifyReviewFinished', next ? 'true' : 'false')
  }
}

export const settings = new SettingsStore()
