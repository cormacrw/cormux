import type { EngineKind, SettingRow } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import {
  DEFAULT_WORKTREE_ROOT,
  readBooleanSetting,
  readDefaultEngine,
  readStringSetting,
} from '$lib/new-workspace/settings-defaults'
import {
  BANGS_KEY,
  LEGACY_MACROS_KEY,
  parseBangs,
  serializeBangs,
  type Bang,
} from '$lib/settings/bangs'
import { deepEqual } from '$lib/deep-equal'
import { reuseRows } from './reuse-rows'

const PR_PROMPT_KEY = 'prPrompt'

function upsertRow(rows: SettingRow[], key: string, value: string) {
  const next = rows.filter((row) => row.key !== key)
  next.push({ key, value })
  return next
}

export class SettingsStore {
  reduceMotion = $state(false)
  runEverything = $state(false)
  defaultEngine = $state<EngineKind>('claude')
  /** Preferred repo id; resolve with `resolveDefaultRepoId` since it may be removed. */
  defaultRepo = $state('')
  /** Experimental: agents pin PRs, tickets and docs to a panel beside the thread. */
  experimentalReferences = $state(false)
  worktreeRoot = $state(DEFAULT_WORKTREE_ROOT)
  notifyApprovals = $state(true)
  notifyReviewFinished = $state(true)
  /** `!name` prompts, for any prompt bar and the command palette (Settings › Bangs). */
  bangs = $state<Bang[]>([])
  /** Instructions for drafting a PR description; empty means the built-in default. */
  prPrompt = $state('')
  /** App that Open in Terminal launches; empty means Terminal. */
  terminalApp = $state('')
  // Raw and reused: readers key off the whole list, so an unchanged snapshot must keep it.
  rows = $state.raw<SettingRow[]>([])
  /** When set, Settings view scrolls/focuses this section (palette deep links). */
  focusSection = $state<string | null>(null)
  /** Expand this repo and focus its run command field (Output empty state). */
  focusRepoRunCommand = $state<string | null>(null)
  /** Expand this repo's config panel (add repo, deep links). */
  expandRepoId = $state<string | null>(null)
  /** Focus Worktree setup after expanding (add repo). */
  focusRepoSetup = $state<string | null>(null)

  // `autoApproveReadOnly` is still sent by the snapshot mapper but nothing reads it.
  hydrate(
    settings: { reduceMotion?: boolean; autoApproveReadOnly?: boolean },
    rows: SettingRow[] = [],
  ) {
    if (settings.reduceMotion !== undefined) {
      this.reduceMotion = settings.reduceMotion
    }
    const previous = this.rows
    this.rows = reuseRows(previous, rows, (row) => row.key)
    if (this.rows === previous) return
    if (rows.length > 0) {
      this.defaultEngine = readDefaultEngine(rows)
      this.defaultRepo = readStringSetting(rows, 'defaultRepo', '')
      this.experimentalReferences = readBooleanSetting(
        rows,
        'experimentalReferences',
        false,
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
      this.prPrompt = readStringSetting(rows, PR_PROMPT_KEY, '')
      this.terminalApp = readStringSetting(rows, 'terminalApp', '')
      const bangs = parseBangs(
        rows.find((row) => row.key === BANGS_KEY)?.value,
        rows.find((row) => row.key === LEGACY_MACROS_KEY)?.value,
      )
      if (!deepEqual(bangs, this.bangs)) this.bangs = bangs
    }
  }

  async persist(key: string, value: string) {
    this.rows = upsertRow(this.rows, key, value)
    const result = await commands.setSetting({ key, value })
    if (result.status === 'error') {
      toastCoreError(result.error)
    }
  }

  async setDefaultEngine(engine: EngineKind) {
    this.defaultEngine = engine
    await this.persist('defaultEngine', engine)
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

  async setRunEverything(next: boolean) {
    this.runEverything = next
    await this.persist('runEverything', next ? 'true' : 'false')
  }

  async setExperimentalReferences(next: boolean) {
    this.experimentalReferences = next
    await this.persist('experimentalReferences', next ? 'true' : 'false')
  }

  async setWorktreeRoot(path: string) {
    const trimmed = path.trim() || DEFAULT_WORKTREE_ROOT
    this.worktreeRoot = trimmed
    await this.persist('worktreeRoot', trimmed)
  }

  async setTerminalApp(name: string) {
    this.terminalApp = name.trim()
    await this.persist('terminalApp', this.terminalApp)
  }

  async setNotifyApprovals(next: boolean) {
    this.notifyApprovals = next
    await this.persist('notifyApprovals', next ? 'true' : 'false')
  }

  async setBangs(next: Bang[]) {
    this.bangs = next
    await this.persist(BANGS_KEY, serializeBangs(next))
  }

  async setPrPrompt(next: string) {
    this.prPrompt = next
    await this.persist(PR_PROMPT_KEY, next)
  }

  async setNotifyReviewFinished(next: boolean) {
    this.notifyReviewFinished = next
    await this.persist('notifyReviewFinished', next ? 'true' : 'false')
  }
}

export const settings = new SettingsStore()
