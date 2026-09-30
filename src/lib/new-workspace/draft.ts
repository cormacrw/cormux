function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 5)
    .join('-')
}

function branchPrefix(text: string): 'feat' | 'fix' | 'refactor' {
  if (/\b(fix|bug|broken|error|crash|repair)\b/i.test(text)) return 'fix'
  if (/\b(refactor|clean|rename)\b/i.test(text)) return 'refactor'
  return 'feat'
}

/** Branch when the prompt did not draft one. Skips names already taken. */
export function fallbackBranchName(taken: string[]): string {
  let name = 'feat/workspace'
  let n = 2
  while (taken.includes(name)) {
    name = `feat/workspace-${n}`
    n += 1
  }
  return name
}

/** Draft branch name from initial prompt (spec §06). */
export function draftBranchName(prompt: string): string {
  const trimmed = prompt.trim()
  if (!trimmed) return ''

  const prefix = branchPrefix(trimmed)
  const slug = slugify(trimmed)
  return `${prefix}/${slug || 'task'}`
}
