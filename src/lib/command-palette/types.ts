export type PaletteIcon =
  | 'bang'
  | 'findings'
  | 'pull'
  | 'restart'
  | 'run'
  | 'settings'
  | 'sprint'
  | 'stop'
  | 'thread'
  | 'workspace'

export type PaletteReference = { name: string; url: string }

export type PaletteCommand = {
  id: string
  /** A bang completes into the field when picked, rather than running. */
  kind?: 'bang'
  group: string
  label: string
  meta?: string
  /** A quieter second line under the label. */
  subtitle?: string
  kbd?: string
  /**
   * What a query can also find this by, without it being shown: a thread's references (the PRs,
   * tickets and docs its agent pinned). The one a query matched shows under the label.
   */
  references?: PaletteReference[]
  icon?: PaletteIcon
  run: () => void
}
