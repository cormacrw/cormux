import type { Skill } from '$lib/ipc/bindings'
import { prefixFirst } from './match'

/** Names starting with the query first, then names or descriptions containing it. */
export function filterSkills(skills: Skill[], query: string): Skill[] {
  return prefixFirst(
    skills,
    query,
    // A plugin skill (`plugin:skill`) also starts where its own name does.
    (skill, needle) => {
      const name = skill.name.toLowerCase()
      return (
        name.startsWith(needle) ||
        name.slice(name.indexOf(':') + 1).startsWith(needle)
      )
    },
    (skill, needle) =>
      skill.name.toLowerCase().includes(needle) ||
      skill.description.toLowerCase().includes(needle),
  )
}
