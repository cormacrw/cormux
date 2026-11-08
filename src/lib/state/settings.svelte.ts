import type { Settings, SettingsPatch } from '$lib/ipc/bindings'
import { commands } from '$lib/ipc'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { deepEqual } from '$lib/deep-equal'

/** What shows for the moment before the first snapshot; the core's settings replace it. */
const BEFORE_FIRST_SNAPSHOT: Settings = {
  reduceMotion: false,
  runEverything: false,
  experimentalReferences: false,
  notifyApprovals: true,
  notifyReviewFinished: true,
  defaultEngine: 'claude',
  defaultRepo: '',
  worktreeRoot: '',
  terminalApp: '',
  prPrompt: '',
  bangs: [],
  clickupWorkspaceId: '',
  clickupSpaceId: '',
  clickupFolderId: '',
  clickupHiddenStatuses: [],
}

function applied(settings: Settings, patch: SettingsPatch): Settings {
  const changes = Object.fromEntries(
    Object.entries(patch).filter(([, value]) => value != null),
  )
  return { ...settings, ...changes }
}

export class SettingsStore {
  // Raw and replaced whole: readers see a new object only when a setting changed.
  values = $state.raw<Settings>(BEFORE_FIRST_SNAPSHOT)
  /** When set, Settings view scrolls/focuses this section (palette deep links). */
  focusSection = $state<string | null>(null)
  /** Expand this repo and focus its run command field (Output empty state). */
  focusRepoRunCommand = $state<string | null>(null)
  /** Expand this repo's config panel (add repo, deep links). */
  expandRepoId = $state<string | null>(null)
  /** Focus Worktree setup after expanding (add repo). */
  focusRepoSetup = $state<string | null>(null)

  get reduceMotion() {
    return this.values.reduceMotion
  }
  get runEverything() {
    return this.values.runEverything
  }
  get defaultEngine() {
    return this.values.defaultEngine
  }
  /** Preferred repo id; resolve with `resolveDefaultRepoId` since it may be removed. */
  get defaultRepo() {
    return this.values.defaultRepo
  }
  /** Experimental: agents pin PRs, tickets and docs to a panel beside the thread. */
  get experimentalReferences() {
    return this.values.experimentalReferences
  }
  get worktreeRoot() {
    return this.values.worktreeRoot
  }
  get notifyApprovals() {
    return this.values.notifyApprovals
  }
  get notifyReviewFinished() {
    return this.values.notifyReviewFinished
  }
  /** `!name` prompts, for any prompt bar and the command palette (Settings › Bangs). */
  get bangs() {
    return this.values.bangs
  }
  /** Instructions for drafting a PR description; empty means the built-in default. */
  get prPrompt() {
    return this.values.prPrompt
  }
  /** App that Open in Terminal launches; empty means Terminal. */
  get terminalApp() {
    return this.values.terminalApp
  }

  hydrate(settings: Settings) {
    if (!deepEqual(settings, this.values)) this.values = settings
  }

  /** Shows the change at once, then saves it; the core's answer is what stays. */
  async update(patch: SettingsPatch) {
    this.values = applied(this.values, patch)
    const result = await commands.updateSettings(patch)
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    this.hydrate(result.data)
  }
}

export const settings = new SettingsStore()
