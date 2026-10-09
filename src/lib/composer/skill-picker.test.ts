import { describe, expect, it } from 'vitest'
import type { Skill } from '$lib/ipc/bindings'
import { filterSkills, insertSkill, skillToken } from './skill-picker'

const skill = (name: string, description = ''): Skill => ({
  name,
  description,
  source: 'user',
})

describe('skillToken', () => {
  const at = (draft: string) => skillToken(draft, draft.length)

  it('reads the slash word before the caret, anywhere in the draft', () => {
    expect(at('/')).toEqual({ start: 0, end: 1, query: '' })
    expect(at('/dep')?.query).toBe('dep')
    expect(at('please run /dep')).toEqual({ start: 11, end: 15, query: 'dep' })
    expect(at('first line\n/rev')?.query).toBe('rev')
  })

  it('covers the rest of the word when the caret is inside it', () => {
    expect(skillToken('/deploy now', 4)).toEqual({
      start: 0,
      end: 7,
      query: 'dep',
    })
  })

  it('ignores paths and words that are not a slash word', () => {
    expect(at('')).toBeNull()
    expect(at('hi')).toBeNull()
    expect(at('src/app')).toBeNull()
    expect(at('/usr/bin')).toBeNull()
    expect(skillToken('/usr/bin', 2)).toBeNull()
    expect(at('/deploy now')).toBeNull()
  })
})

describe('insertSkill', () => {
  it('replaces just the slash word and leaves the caret after it', () => {
    const draft = 'please /rev the diff'
    const token = skillToken(draft, 11)!
    expect(insertSkill(draft, token, 'code-review')).toEqual({
      text: 'please /code-review the diff',
      caret: 20,
    })
    expect(insertSkill('/de', skillToken('/de', 3)!, 'deploy')).toEqual({
      text: '/deploy ',
      caret: 8,
    })
  })
})

describe('filterSkills', () => {
  it('ranks name prefixes before other matches', () => {
    const skills = [
      skill('code-review'),
      skill('deploy', 'Ship to prod'),
      skill('review'),
    ]
    expect(filterSkills(skills, 'rev').map((row) => row.name)).toEqual([
      'review',
      'code-review',
    ])
    expect(filterSkills(skills, 'prod').map((row) => row.name)).toEqual([
      'deploy',
    ])
    expect(filterSkills(skills, '')).toHaveLength(3)
  })

  it('matches a plugin skill by its own name', () => {
    const skills = [skill('pdf-tools-guide', 'about docx'), skill('acme:docx')]
    expect(filterSkills(skills, 'docx').map((row) => row.name)).toEqual([
      'acme:docx',
      'pdf-tools-guide',
    ])
  })
})
