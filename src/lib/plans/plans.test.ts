import { describe, expect, it } from 'vitest'
import type { ApprovalRow } from '$lib/ipc/bindings'
import { formatPlanFeedback } from './feedback'
import {
  hiddenPlanIds,
  latestPlanPerThread,
  plansFromApprovals,
  threadPlanMode,
} from './plans'

function approval(
  id: string,
  threadId: string,
  status: string,
  extra: Record<string, unknown> = {},
): ApprovalRow {
  return {
    id,
    threadId,
    status,
    tool: 'execute_plan',
    payload: JSON.stringify({
      title: 'Review plan',
      what: `Plan ${id}`,
      why: '',
      okLabel: 'Approve plan',
      noLabel: 'Sent feedback',
      effect: 'execute_plan',
      plan: `# Plan ${id}`,
      ...extra,
    }),
  }
}

describe('plansFromApprovals', () => {
  it('keeps plan approvals, numbering each thread’s revisions', () => {
    const plans = plansFromApprovals([
      approval('a', 't1', 'denied', { denyReason: 'Cap retries' }),
      {
        id: 'bash',
        threadId: 't1',
        status: 'pending',
        tool: 'execute',
        payload: JSON.stringify({
          title: 'Use Bash',
          what: 'ls',
          why: '',
          okLabel: 'Approve',
          noLabel: 'Deny',
        }),
      },
      approval('b', 't2', 'approved'),
      approval('c', 't1', 'pending'),
    ])
    expect(
      plans.map((plan) => [plan.id, plan.state, plan.version, plan.feedback]),
    ).toEqual([
      ['a', 'revised', 1, 'Cap retries'],
      ['b', 'approved', 1, null],
      ['c', 'pending', 2, null],
    ])
    expect(plans[2]).toMatchObject({ title: 'Plan c', markdown: '# Plan c' })
  })

  it('skips plan approvals without a plan to show', () => {
    expect(
      plansFromApprovals([approval('a', 't1', 'pending', { plan: '' })]),
    ).toEqual([])
  })
})

describe('latestPlanPerThread', () => {
  it('returns each thread’s newest plan in thread order', () => {
    const plans = plansFromApprovals([
      approval('a', 't1', 'denied'),
      approval('b', 't2', 'pending'),
      approval('c', 't1', 'pending'),
    ])
    expect(
      latestPlanPerThread(plans, ['t2', 't1', 't3']).map((plan) => plan.id),
    ).toEqual(['b', 'c'])
  })
})

describe('dismissed and hidden plans', () => {
  it('reads a plan the user left plan mode on as dismissed', () => {
    expect(
      plansFromApprovals([approval('a', 't1', 'dismissed')])[0]?.state,
    ).toBe('dismissed')
  })

  it('reads the plans closed in a workspace, ignoring a broken setting', () => {
    const rows = [
      { key: 'workspace:w1/hiddenPlans', value: '["a","b"]' },
      { key: 'workspace:w2/hiddenPlans', value: 'nope' },
    ]
    expect(hiddenPlanIds(rows, 'w1')).toEqual(['a', 'b'])
    expect(hiddenPlanIds(rows, 'w2')).toEqual([])
    expect(hiddenPlanIds(rows, 'w3')).toEqual([])
  })
})

describe('threadPlanMode', () => {
  it('reads the per-thread setting', () => {
    const rows = [
      { key: 'thread:t1/planMode', value: 'true' },
      { key: 'thread:t2/planMode', value: 'false' },
    ]
    expect(threadPlanMode(rows, 't1')).toBe(true)
    expect(threadPlanMode(rows, 't2')).toBe(false)
    expect(threadPlanMode(rows, 't3')).toBe(false)
  })
})

describe('formatPlanFeedback', () => {
  it('quotes each comment in plan order, then the overall note', () => {
    const text = formatPlanFeedback(
      [
        { id: '2', start: 40, end: 50, quote: 'Add a test', body: 'Two tests' },
        {
          id: '1',
          start: 5,
          end: 20,
          quote: 'Retry the call\n\n  forever  ',
          body: ' Cap it at 3 ',
        },
      ],
      'Looks close',
    )
    expect(text).toBe(
      [
        "I reviewed your plan. Revise it with this feedback, then present the updated plan. Don't start editing yet.",
        '',
        '> Retry the call\n> forever\nCap it at 3',
        '',
        '> Add a test\nTwo tests',
        '',
        'Overall: Looks close',
      ].join('\n'),
    )
  })

  it('sends a note on its own when there are no comments', () => {
    expect(
      formatPlanFeedback([], 'Start over').endsWith('\n\nStart over'),
    ).toBe(true)
  })

  it('clips a very long quote', () => {
    const text = formatPlanFeedback(
      [
        {
          id: '1',
          start: 0,
          end: 900,
          quote: 'x'.repeat(900),
          body: 'Shorter',
        },
      ],
      '',
    )
    expect(text).toContain(`> ${'x'.repeat(400)}…\nShorter`)
  })
})
