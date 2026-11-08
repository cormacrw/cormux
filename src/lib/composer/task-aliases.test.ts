import { describe, expect, it } from 'vitest'
import { expandTaskAliases, filterTasks, taskContext } from './task-aliases'

const tasks = [
  {
    id: '86b1x2k',
    customId: 'ENG-123',
    name: 'Retry failed webhooks',
    url: 'https://app.clickup.com/t/86b1x2k',
  },
  {
    id: '86b1x9q',
    customId: null,
    name: 'Fix login redirect',
    url: 'https://app.clickup.com/t/86b1x9q',
  },
]

describe('task aliases', () => {
  it('matches the custom ID or ClickUp’s own, then the name', () => {
    expect(filterTasks(tasks, 'eng').map((task) => task.id)).toEqual([
      '86b1x2k',
    ])
    expect(filterTasks(tasks, '86b1x9').map((task) => task.id)).toEqual([
      '86b1x9q',
    ])
    expect(filterTasks(tasks, 'login').map((task) => task.id)).toEqual([
      '86b1x9q',
    ])
    expect(filterTasks(tasks, '')).toHaveLength(2)
  })

  it('expands known references and leaves other # words alone', () => {
    const { text, mentioned } = expandTaskAliases(
      'Do #ENG-123 then #86b1x9q, and #eng-123 again. See PR #482.\n# Notes',
      tasks,
    )
    expect(text).toBe(
      'Do ENG-123 “Retry failed webhooks” (https://app.clickup.com/t/86b1x2k) then 86b1x9q “Fix login redirect” (https://app.clickup.com/t/86b1x9q), and ENG-123 “Retry failed webhooks” (https://app.clickup.com/t/86b1x2k) again. See PR #482.\n# Notes',
    )
    expect(mentioned.map((task) => task.id)).toEqual(['86b1x2k', '86b1x9q'])
    expect(expandTaskAliases('#ENG-123', [])).toEqual({
      text: '#ENG-123',
      mentioned: [],
    })
  })

  it('adds the descriptions it has, clipped', () => {
    expect(
      taskContext(tasks, {
        '86b1x2k': 'Webhooks retry forever.\n\nCap at 3.',
        '86b1x9q': '   ',
      }),
    ).toBe(
      'ClickUp tasks:\n\n### ENG-123 Retry failed webhooks\nWebhooks retry forever.\n\nCap at 3.',
    )
    expect(taskContext(tasks, {})).toBe('')
    expect(taskContext([tasks[0]!], { '86b1x2k': 'x'.repeat(2500) })).toContain(
      `${'x'.repeat(2000)}…`,
    )
  })
})
