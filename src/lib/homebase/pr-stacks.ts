type StackablePr = {
  id: string
  head: string
  base: string
  repoFullName: string
}

export type PrListEntry<T> =
  | { kind: 'single'; pr: T }
  /** Top of the stack first, down to the PR that targets the trunk. */
  | { kind: 'stack'; id: string; trunk: string; prs: T[] }

const branchKey = (repo: string, branch: string) => `${repo}\u0000${branch}`

/**
 * Groups PRs that target each other's branches (one PR's base is another's head, in the
 * same repo) into stacks. Each group takes the place of its first PR in `prs`, so the
 * list keeps its order.
 */
export function groupPrsByStack<T extends StackablePr>(
  prs: T[],
): PrListEntry<T>[] {
  const byHead = new Map<string, T>()
  for (const pr of prs) byHead.set(branchKey(pr.repoFullName, pr.head), pr)
  const parentOf = (pr: T) => {
    const parent = byHead.get(branchKey(pr.repoFullName, pr.base))
    return parent && parent !== pr ? parent : undefined
  }

  // Walk down to the PR that targets a branch with no open PR; that's the stack's root.
  const rootOf = new Map<T, T>()
  const depthOf = new Map<T, number>()
  for (const pr of prs) {
    let root = pr
    let depth = 0
    const seen = new Set<T>([pr])
    for (let parent = parentOf(root); parent; parent = parentOf(root)) {
      // A cycle (two PRs targeting each other) has no root; stop where it loops.
      if (seen.has(parent)) break
      seen.add(parent)
      root = parent
      depth += 1
    }
    rootOf.set(pr, root)
    depthOf.set(pr, depth)
  }

  const members = new Map<T, T[]>()
  for (const pr of prs) {
    const root = rootOf.get(pr) ?? pr
    members.set(root, [...(members.get(root) ?? []), pr])
  }

  const entries: PrListEntry<T>[] = []
  const placed = new Set<T>()
  for (const pr of prs) {
    const root = rootOf.get(pr) ?? pr
    if (placed.has(root)) continue
    placed.add(root)
    const group = members.get(root) ?? [pr]
    if (group.length === 1) {
      entries.push({ kind: 'single', pr })
      continue
    }
    entries.push({
      kind: 'stack',
      id: root.id,
      trunk: root.base,
      prs: [...group].sort(
        (a, b) => (depthOf.get(b) ?? 0) - (depthOf.get(a) ?? 0),
      ),
    })
  }
  return entries
}

export type PrCard<T> =
  | { kind: 'list'; id: string; prs: T[] }
  | Extract<PrListEntry<T>, { kind: 'stack' }>

/** One card per stack, with the standalone PRs between stacks sharing a card. */
export function prCards<T extends StackablePr>(
  entries: PrListEntry<T>[],
): PrCard<T>[] {
  const cards: PrCard<T>[] = []
  for (const entry of entries) {
    if (entry.kind === 'stack') {
      cards.push(entry)
      continue
    }
    const last = cards.at(-1)
    if (last?.kind === 'list') last.prs.push(entry.pr)
    else cards.push({ kind: 'list', id: entry.pr.id, prs: [entry.pr] })
  }
  return cards
}
