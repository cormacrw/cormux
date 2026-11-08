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
  icon?: PaletteIcon
  run: () => void
}
