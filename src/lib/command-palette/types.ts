import type { Component } from 'svelte'

export type PaletteCommand = {
  id: string
  group: string
  label: string
  meta?: string
  kbd?: string
  icon?: Component
  run: () => void
}
