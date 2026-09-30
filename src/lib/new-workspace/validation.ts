export type BranchValidationContext = {
  existingBranches: string[]
  workspaceBranches: string[]
  /** The repo's default branch, which stays in its own checkout. */
  defaultBranch?: string
}

export type BranchFieldError =
  'required' | 'format' | 'default' | 'exists' | 'workspaceConflict'

const BRANCH_FORMAT = /^[A-Za-z0-9._/-]+$/

export function validateBranchName(
  branch: string,
  ctx: BranchValidationContext,
): BranchFieldError | null {
  const value = branch.trim()
  if (!value) return 'required'
  if (
    !BRANCH_FORMAT.test(value) ||
    value.includes('//') ||
    value.startsWith('/') ||
    value.endsWith('/') ||
    value.includes('..')
  ) {
    return 'format'
  }
  if (value === ctx.defaultBranch) return 'default'
  if (ctx.existingBranches.includes(value)) return 'exists'
  if (ctx.workspaceBranches.includes(value)) return 'workspaceConflict'
  return null
}

export function branchErrorMessage(
  branch: string,
  code: BranchFieldError,
): string {
  switch (code) {
    case 'required':
      return 'Add a branch name, e.g. feat/oauth-login.'
    case 'format':
      return 'Use letters, numbers, dashes and slashes only, e.g. feat/oauth-login.'
    case 'default':
      return `${branch.trim()} is the repo's default branch. Pick a new branch name.`
    case 'exists':
      return `${branch.trim()} already exists. Pick a new branch name.`
    case 'workspaceConflict':
      return 'Another workspace already uses this branch.'
  }
}

export function validateBaseBranch(
  value: string,
  knownBranches: string[],
): string | null {
  const trimmed = value.trim()
  if (!trimmed) {
    return 'No branch called “…”. Pick one from the list.'
  }
  if (!knownBranches.includes(trimmed)) {
    return `No branch called “${trimmed}”. Pick one from the list.`
  }
  return null
}
