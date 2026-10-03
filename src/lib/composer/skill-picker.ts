import type { Skill } from '$lib/ipc/bindings'

const MAX_SHOWN = 8

/** The skill name being typed: a draft that is just `/` and a word, or null. */
export function skillQuery(draft: string): string | null {
  const match = /^\/([^\s/]*)$/.exec(draft)
  return match ? (match[1] ?? '') : null
}

/** Names starting with the query first, then names or descriptions containing it. */
export function filterSkills(skills: Skill[], query: string): Skill[] {
  const needle = query.toLowerCase()
  const starts = skills.filter((skill) =>
    skill.name.toLowerCase().startsWith(needle),
  )
  const contains = skills.filter(
    (skill) =>
      !starts.includes(skill) &&
      (skill.name.toLowerCase().includes(needle) ||
        skill.description.toLowerCase().includes(needle)),
  )
  return [...starts, ...contains].slice(0, MAX_SHOWN)
}
