import type { PaletteCommand } from './types'

/** Case-insensitive substring match on label and meta (spec §02). */
export function commandMatchesQuery(
  command: PaletteCommand,
  query: string,
): boolean {
  const q = query.trim().toLowerCase()
  if (!q) return true
  if (command.label.toLowerCase().includes(q)) return true
  if (command.meta?.toLowerCase().includes(q)) return true
  return false
}

export function filterCommands(
  commands: PaletteCommand[],
  query: string,
): PaletteCommand[] {
  return commands.filter((command) => commandMatchesQuery(command, query))
}

export const GROUP_ORDER = [
  'Actions',
  'Workspaces',
  'Scratches',
  'Threads',
  'App',
  'Git',
] as const

export type PaletteGroup = (typeof GROUP_ORDER)[number]

export function groupFilteredCommands(commands: PaletteCommand[]) {
  const buckets = new Map<string, PaletteCommand[]>()
  for (const command of commands) {
    const list = buckets.get(command.group) ?? []
    list.push(command)
    buckets.set(command.group, list)
  }
  const ordered: { group: string; items: PaletteCommand[] }[] = []
  for (const group of GROUP_ORDER) {
    const items = buckets.get(group)
    if (items?.length) ordered.push({ group, items })
  }
  for (const [group, items] of buckets) {
    if (GROUP_ORDER.includes(group as PaletteGroup)) continue
    if (items.length) ordered.push({ group, items })
  }
  return ordered
}
