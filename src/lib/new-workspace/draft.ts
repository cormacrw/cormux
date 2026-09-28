/** Draft workspace title from initial prompt (spec §06). */
export function draftWorkspaceName(prompt: string): string {
  const trimmed = prompt.trim()
  if (!trimmed) return ''

  const words = trimmed.replace(/[.\s]+$/, '').split(/\s+/)
  let out = ''
  for (const word of words) {
    const next = (out ? `${out} ${word}` : word).trim()
    if (next.length > 40) break
    out = next
  }
  if (!out) return ''
  return out.charAt(0).toUpperCase() + out.slice(1)
}

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

/** Draft branch name from initial prompt (spec §06). */
export function draftBranchName(prompt: string): string {
  const trimmed = prompt.trim()
  if (!trimmed) return ''

  const prefix = branchPrefix(trimmed)
  const slug = slugify(trimmed)
  return `${prefix}/${slug || 'task'}`
}
