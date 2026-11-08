import type { Settings, SettingsPatch } from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import { onSnapshot } from '$lib/core/sync'
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

  /** Shows the change at once, then saves it; the core's answer is what stays. Resolves why it didn't save. */
  async update(patch: SettingsPatch): Promise<CoreError | null> {
    this.values = applied(this.values, patch)
    const result = await commands.updateSettings(patch)
    if (result.status === 'error') return result.error
    this.hydrate(result.data)
    return null
  }
}

export const settings = new SettingsStore()

onSnapshot((snapshot) => settings.hydrate(snapshot.settings))
