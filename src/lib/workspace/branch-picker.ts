import type { RepoBranch } from '$lib/ipc/bindings'
import { branchHoldLabel } from '$lib/repo/branch-hold'

type BranchPickerItem = {
  name: string
  /** A few words after the name: why it can't be picked, or that it's this one. */
  meta: string | null
  disabled: boolean
  checked: boolean
}

/** The checked-out branch first, then the repo's, held ones disabled by the core's holds. */
export function buildBranchPickerList(input: {
  current: string
  branches: RepoBranch[]
}): BranchPickerItem[] {
  const others = input.branches
    .filter((branch) => branch.name !== input.current)
    .map((branch) => ({
      name: branch.name,
      meta: branch.hold ? branchHoldLabel(branch.hold) : null,
      disabled: branch.hold != null,
      checked: false,
    }))
  return [
    { name: input.current, meta: 'current', disabled: true, checked: true },
    ...others,
  ]
}
