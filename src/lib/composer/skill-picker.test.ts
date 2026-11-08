import { describe, expect, it } from 'vitest'
import type { Skill } from '$lib/ipc/bindings'
import { filterSkills } from './skill-picker'

const skill = (name: string, description = ''): Skill => ({
  name,
  description,
  source: 'user',
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
