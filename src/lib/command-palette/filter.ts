import type { PaletteCommand } from './types'

const haystacks = new WeakMap<PaletteCommand, string>()

function haystack(command: PaletteCommand): string {
  let text = haystacks.get(command)
  if (text === undefined) {
    text = command.meta
      ? `${command.label}\n${command.meta}`.toLowerCase()
      : command.label.toLowerCase()
    haystacks.set(command, text)
  }
  return text
}

/** Case-insensitive substring match on label and meta (product/features/02-command-palette-and-shortcuts.md). */
export function filterCommands(
  commands: PaletteCommand[],
  query: string,
): PaletteCommand[] {
  const q = query.trim().toLowerCase()
  if (!q) return commands
  return commands.filter((command) => haystack(command).includes(q))
}

const GROUP_ORDER = [
  'Actions',
  'Bangs',
  'Workspaces',
  'Scratches',
  'Threads',
  'App',
  'Git',
] as const

type PaletteGroup = (typeof GROUP_ORDER)[number]

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
