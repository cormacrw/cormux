import { describe, expect, it } from 'vitest'
import { todoAliasTask } from '$lib/todos/ref'
import {
  todoContext,
  expandTaskAliases,
  filterTasks,
  insertTaskAlias,
  taskContext,
  taskToken,
} from './task-aliases'

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
  it('finds the # word the caret is in', () => {
    expect(taskToken('work on #eng', 12)).toEqual({
      start: 8,
      end: 12,
      query: 'eng',
    })
    expect(taskToken('#', 1)).toEqual({ start: 0, end: 1, query: '' })
    expect(taskToken('issue#12', 8)).toBeNull()
    expect(taskToken('# Heading', 9)).toBeNull()
  })

  it('inserts the reference with a space after it', () => {
    expect(
      insertTaskAlias('do #en', { start: 3, end: 6, query: 'en' }, 'ENG-123'),
    ).toEqual({ text: 'do #ENG-123 ', caret: 12 })
  })

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

describe('TODO aliases', () => {
  const todo = todoAliasTask(
    {
      id: 'todo-1',
      number: 12,
      title: 'Book the venue',
      pinned: false,
      description: 'Ask for the big room',
      plannedOn: null,
      doneAt: null,
    },
    '/data/tasks/',
  )

  it('expands #CMUX-n to its ref, title and file, like a ClickUp task', () => {
    const { text, mentioned } = expandTaskAliases('Do #cmux-12 today', [todo])
    expect(text).toBe(
      'Do CMUX-12 “Book the venue” (/data/tasks/CMUX-12.md) today',
    )
    expect(mentioned).toEqual([todo])
  })

  it('tells the agent where to read each TODO', () => {
    expect(todoContext([todo])).toBe(
      "Cormux TODOs. Read each file for the task's notes and status:\n- CMUX-12 “Book the venue”: /data/tasks/CMUX-12.md",
    )
    expect(todoContext([])).toBe('')
  })
})
