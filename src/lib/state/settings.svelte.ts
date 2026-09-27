export class SettingsStore {
  reduceMotion = $state(false)
  autoApproveReadOnly = $state(true)

  hydrate(settings: { reduceMotion?: boolean; autoApproveReadOnly?: boolean }) {
    if (settings.reduceMotion !== undefined) {
      this.reduceMotion = settings.reduceMotion
    }
    if (settings.autoApproveReadOnly !== undefined) {
      this.autoApproveReadOnly = settings.autoApproveReadOnly
    }
  }
}

export const settings = new SettingsStore()
