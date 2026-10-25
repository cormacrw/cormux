import { describe, expect, it } from 'vitest'
import { DEFAULT_TASK_PROMPT, taskPrompt } from './task-prompt'

describe('taskPrompt', () => {
  it('refers to a task by #ref, so sending expands it', () => {
    expect(taskPrompt({ tag: 'CU-42' })).toBe(
      'Complete #CU-42: read the task, make the change, and check it works.',
    )
    expect(taskPrompt({ tag: 'CMUX-7' })).toBe(
      'Complete #CMUX-7: read the task, make the change, and check it works.',
    )
  })

  it('follows a custom template, or adds the task when it has no {task}', () => {
    expect(taskPrompt({ tag: 'CU-42' }, 'Plan {task} first.')).toBe(
      'Plan #CU-42 first.',
    )
    expect(taskPrompt({ tag: 'CU-42' }, 'Work on')).toBe('Work on #CU-42')
    expect(taskPrompt({ tag: 'CU-42' }, '  ')).toBe(
      DEFAULT_TASK_PROMPT.replace('{task}', '#CU-42'),
    )
  })
})
