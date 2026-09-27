export class SettingsStore {
  reduceMotion = $state(false)
  autoApproveReadOnly = $state(true)
  /** When set, Settings view scrolls/focuses this section (palette deep links). */
  focusSection = $state<string | null>(null)

  hydrate(settings: { reduceMotion?: boolean; autoApproveReadOnly?: boolean }) {
    if (settings.reduceMotion !== undefined) {
      this.reduceMotion = settings.reduceMotion
    }
    if (settings.autoApproveReadOnly !== undefined) {
      this.autoApproveReadOnly = settings.autoApproveReadOnly
    }
  }

  toggleReduceMotion() {
    this.reduceMotion = !this.reduceMotion
  }
}

export const settings = new SettingsStore()
