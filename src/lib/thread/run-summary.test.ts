import { describe, expect, it } from 'vitest'
import { runSucceeded, runSummaryLabel, summarizeRun } from './run-summary'

describe('summarizeRun', () => {
  it('reads vitest, ignoring colour codes', () => {
    const summary = summarizeRun(
      ' \u001b[31mFAIL\u001b[0m src/a.test.ts > adds\n      Tests  1 failed | 268 passed (269)\n',
    )
    expect(summary).toMatchObject({ passed: 268, failed: 1 })
    expect(summary?.failures).toEqual(['src/a.test.ts > adds'])
  })

  it('adds up every cargo test binary', () => {
    const summary = summarizeRun(
      'test result: ok. 152 passed; 0 failed; 3 ignored; 0 measured\n\ntest result: ok. 4 passed; 0 failed; 0 ignored; 0 measured\n',
    )
    expect(summary).toMatchObject({ passed: 156, failed: 0, skipped: 3 })
  })

  it('reads Playwright and pytest', () => {
    expect(summarizeRun('  1 failed\n  90 passed (60.0s)\n')).toMatchObject({
      passed: 90,
      failed: 1,
    })
    expect(summarizeRun('=== 2 failed, 10 passed in 1.20s ===')).toMatchObject({
      passed: 10,
      failed: 2,
    })
  })

  it('counts compiler errors', () => {
    const summary = summarizeRun(
      'error[E0425]: cannot find function\nsrc/a.ts(1,1): error TS2304: nope\n',
    )
    expect(summary?.errors).toBe(2)
    expect(runSucceeded(summary!)).toBe(false)
  })

  it('leaves other output alone', () => {
    expect(summarizeRun('Compiling cormux\nDone in 2s\n')).toBeNull()
    expect(
      summarizeRun('It passed review and 3 people failed to notice'),
    ).toBeNull()
  })
})

describe('runSummaryLabel', () => {
  it('names only what happened', () => {
    expect(
      runSummaryLabel({
        passed: 3,
        failed: 1,
        skipped: 0,
        errors: 0,
        failures: [],
      }),
    ).toBe('3 passed, 1 failed')
    expect(
      runSummaryLabel({
        passed: 0,
        failed: 0,
        skipped: 0,
        errors: 1,
        failures: [],
      }),
    ).toBe('1 error')
  })
})
