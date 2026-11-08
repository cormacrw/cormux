import { describe, expect, it } from 'vitest'
import { approvalResolvedChip, composeAnswer } from './payload'

describe('composeAnswer', () => {
  it('keeps picked options and typed text together', () => {
    expect(composeAnswer(['Red'], '')).toBe('Red')
    expect(composeAnswer(['Red', 'Blue'], '  but darker ')).toBe(
      'Red, Blue, but darker',
    )
    expect(composeAnswer([], 'Green')).toBe('Green')
    expect(composeAnswer([], '   ')).toBe('')
  })
})

describe('approvalResolvedChip', () => {
  const now = Date.now()
  const card = {
    noLabel: 'Deny',
    doneAtMs: now - 1000,
    questions: [] as unknown[],
  }

  it('says who approved a permission, or the denial in its own words', () => {
    expect(approvalResolvedChip({ ...card, state: 'approved' }, now)).toBe(
      'Approved by you · just now',
    )
    expect(approvalResolvedChip({ ...card, state: 'denied' }, now)).toBe(
      'Deny · just now',
    )
  })

  it('says a question was answered or skipped', () => {
    const question = { ...card, questions: [{}] }
    expect(approvalResolvedChip({ ...question, state: 'approved' }, now)).toBe(
      'Answered · just now',
    )
    expect(
      approvalResolvedChip(
        { ...question, state: 'denied', doneAtMs: now - 3 * 60_000 },
        now,
      ),
    ).toBe('Skipped · 3m ago')
  })
})
