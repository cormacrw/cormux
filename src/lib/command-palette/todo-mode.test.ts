import { describe, expect, it } from 'vitest'
import { isTodoTrigger, todoTitleFromQuery } from './todo-mode'

describe('todo palette mode', () => {
  it('triggers on the bare word, any case', () => {
    expect(isTodoTrigger('todo')).toBe(true)
    expect(isTodoTrigger(' TODO ')).toBe(true)
    expect(isTodoTrigger('todos')).toBe(false)
    expect(isTodoTrigger('to')).toBe(false)
  })

  it('takes the title from a pasted "todo <title>"', () => {
    expect(todoTitleFromQuery('todo Buy milk')).toBe('Buy milk')
    expect(todoTitleFromQuery('Todo  ship it ')).toBe('ship it ')
    expect(todoTitleFromQuery('todo ')).toBeNull()
    expect(todoTitleFromQuery('todos page')).toBeNull()
  })
})
