import type { Component } from 'svelte'

export type PaletteCommand = {
  id: string
  group: string
  label: string
  meta?: string
  /** A quieter second line under the label. */
  subtitle?: string
  kbd?: string
  icon?: Component
  run: () => void
}
