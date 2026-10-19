import type { Skill } from '$lib/ipc/bindings'

const MAX_SHOWN = 8

/** A `/skill` word in the draft: where it starts and ends, and what's typed before the caret. */
export type SkillToken = { start: number; end: number; query: string }

/**
 * The `/` word the caret is in, when it starts the draft or follows whitespace, or null.
 * A path such as `src/app` or `/usr/bin` doesn't count.
 */
export function skillToken(draft: string, caret: number): SkillToken | null {
  const match = /(^|\s)\/([^\s/]*)$/.exec(draft.slice(0, caret))
  if (!match) return null
  const rest = /^\S*/.exec(draft.slice(caret))?.[0] ?? ''
  if (rest.includes('/')) return null
  return {
    start: match.index + (match[1]?.length ?? 0),
    end: caret + rest.length,
    query: match[2] ?? '',
  }
}

/** The draft with `token` replaced by `/name `, and where the caret goes after it. */
export function insertSkill(
  draft: string,
  token: SkillToken,
  name: string,
): { text: string; caret: number } {
  const rest = draft.slice(token.end)
  const inserted = `/${name}${/^\s/.test(rest) ? '' : ' '}`
  return {
    text: draft.slice(0, token.start) + inserted + rest,
    caret: token.start + name.length + 2,
  }
}

/** Names starting with the query first, then names or descriptions containing it. */
export function filterSkills(skills: Skill[], query: string): Skill[] {
  const needle = query.toLowerCase()
  // A plugin skill (`plugin:skill`) also starts where its own name does.
  const starts = skills.filter((skill) => {
    const name = skill.name.toLowerCase()
    return (
      name.startsWith(needle) ||
      name.slice(name.indexOf(':') + 1).startsWith(needle)
    )
  })
  const contains = skills.filter(
    (skill) =>
      !starts.includes(skill) &&
      (skill.name.toLowerCase().includes(needle) ||
        skill.description.toLowerCase().includes(needle)),
  )
  return [...starts, ...contains].slice(0, MAX_SHOWN)
}
