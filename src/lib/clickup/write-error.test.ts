import { describe, expect, it } from 'vitest'
import { writeErrorText } from './write-error'

describe('writeErrorText', () => {
  it('says what was being tried as well as what ClickUp answered', () => {
    expect(
      writeErrorText({
        error: { kind: 'Clickup', message: 'Status not found' },
        doing: 'Could not move the task to Done',
      }),
    ).toBe('Could not move the task to Done: Status not found')
  })
})
