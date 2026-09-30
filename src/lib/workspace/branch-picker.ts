export type BranchMeta =
  'current' | 'inOtherWorkspace' | 'default' | 'integration' | 'staging'

export type BranchPickerItem = {
  name: string
  meta: BranchMeta | null
  disabled: boolean
  checked: boolean
}

export function branchMetaLabel(meta: BranchMeta | null): string | null {
  if (!meta) return null
  if (meta === 'current') return 'current'
  if (meta === 'inOtherWorkspace') return 'in another workspace'
  if (meta === 'default') return 'default'
  if (meta === 'integration') return 'integration'
  if (meta === 'staging') return 'deploys to staging'
  return null
}

export function metaForBranch(
  branch: string,
  current: string,
  occupied: Set<string>,
): BranchMeta | null {
  if (branch === current) return 'current'
  if (occupied.has(branch)) return 'inOtherWorkspace'
  if (branch === 'main') return 'default'
  if (branch === 'develop') return 'integration'
  if (branch === 'staging') return 'staging'
  return null
}

export function buildBranchPickerList(input: {
  current: string
  repoBranches: string[]
  otherWorkspaceBranches: string[]
}): BranchPickerItem[] {
  const seen = new Set<string>()
  const ordered: string[] = []

  const push = (branch: string) => {
    if (seen.has(branch)) return
    seen.add(branch)
    ordered.push(branch)
  }

  push(input.current)
  for (const branch of input.repoBranches) push(branch)
  for (const branch of input.otherWorkspaceBranches) push(branch)

  const occupied = new Set(
    input.otherWorkspaceBranches.filter((branch) => branch !== input.current),
  )

  return ordered.map((name) => {
    const meta = metaForBranch(name, input.current, occupied)
    return {
      name,
      meta,
      disabled: meta === 'current' || meta === 'inOtherWorkspace',
      checked: name === input.current,
    }
  })
}
