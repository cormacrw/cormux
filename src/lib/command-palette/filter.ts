import type { PaletteCommand, PaletteReference } from './types'

const haystacks = new WeakMap<PaletteCommand, string>()

function haystack(command: PaletteCommand): string {
  let text = haystacks.get(command)
  if (text === undefined) {
    text = normalize(
      command.meta ? `${command.label}\n${command.meta}` : command.label,
    )
    haystacks.set(command, text)
  }
  return text
}

/** `#` is dropped from both sides, so `482`, `#482`, `pr 482` and `PR #482` all find PR #482. */
function normalize(text: string): string {
  return text.toLowerCase().replaceAll('#', '')
}

/** Case-insensitive substring match on label, meta and references (product/features/02-command-palette-and-shortcuts.md). */
export function filterCommands(
  commands: PaletteCommand[],
  query: string,
): PaletteCommand[] {
  const q = normalize(query.trim())
  if (!q) return commands
  return commands.filter(
    (command) =>
      haystack(command).includes(q) || referenceMatching(command, q) != null,
  )
}

const referenceHaystacks = new WeakMap<PaletteReference, string>()

function referenceMatching(
  command: PaletteCommand,
  q: string,
): PaletteReference | undefined {
  return command.references?.find((reference) => {
    let text = referenceHaystacks.get(reference)
    if (text === undefined) {
      text = normalize(`${reference.name}\n${reference.url}`)
      referenceHaystacks.set(reference, text)
    }
    return text.includes(q)
  })
}

/**
 * The reference that put a command in the results, by its name, so it's clear why a thread
 * matched. Nothing when the label or meta matched on their own.
 */
export function matchedReference(
  command: PaletteCommand,
  query: string,
): string | undefined {
  const q = normalize(query.trim())
  if (!q || haystack(command).includes(q)) return undefined
  return referenceMatching(command, q)?.name
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
