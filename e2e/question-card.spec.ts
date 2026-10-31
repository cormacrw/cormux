import { expect, test, type Page } from '@playwright/test'

const QUESTIONS = [
  {
    question: 'Which database should the cache use?',
    header: 'Database',
    options: [
      { label: 'Postgres', description: 'Already running in production' },
      { label: 'Redis', description: 'Faster, one more service to run' },
    ],
    multiSelect: false,
  },
  {
    question: 'Which environments need it?',
    header: 'Environments',
    options: [
      { label: 'Staging', description: '' },
      { label: 'Production', description: '' },
    ],
    multiSelect: true,
  },
]

async function openQuestion(page: Page, questions = QUESTIONS) {
  await page.addInitScript(
    ({ timeline, approvals }) => {
      const win = window as {
        __HARNESS_TIMELINE__?: unknown
        __HARNESS_APPROVALS__?: unknown
      }
      win.__HARNESS_TIMELINE__ = timeline
      win.__HARNESS_APPROVALS__ = approvals
    },
    {
      timeline: [
        {
          id: 9101,
          threadId: 'th-lead',
          seq: 1,
          kind: 'permission',
          payload: JSON.stringify({
            type: 'permission',
            id: 'q1',
            tool_call_id: 'toolu_ask',
            title: 'Use AskUserQuestion',
            tool_name: 'AskUserQuestion',
            kind: 'other',
            detail: null,
            auto_approved: false,
          }),
          createdAt: '2026-10-30 12:00:00',
        },
      ],
      approvals: [
        {
          id: 'q1',
          threadId: 'th-lead',
          status: 'pending',
          tool: 'question',
          payload: JSON.stringify({
            title: 'Question',
            what: questions[0].question,
            why: '',
            okLabel: 'Send',
            noLabel: 'Skip',
            questions,
          }),
        },
      ],
    },
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  const card = page.locator('[data-od-id="question-q1"]')
  await expect(card).toBeVisible()
  return card
}

function answered(page: Page) {
  return page.evaluate(
    () =>
      (window as { __HARNESS_ANSWERED__?: unknown[] }).__HARNESS_ANSWERED__ ??
      [],
  )
}

test('picks options and types alongside them', async ({ page }) => {
  const card = await openQuestion(page)
  const send = card.getByRole('button', { name: 'Send' })
  await expect(send).toBeDisabled()

  const redis = card.getByRole('button', { name: /Redis/ })
  await card.getByRole('button', { name: /Postgres/ }).click()
  await redis.click()
  await expect(redis).toHaveAttribute('aria-pressed', 'true')
  await expect(card.getByRole('button', { name: /Postgres/ })).toHaveAttribute(
    'aria-pressed',
    'false',
  )
  await expect(send).toBeDisabled()

  await card.getByRole('button', { name: /Staging/ }).click()
  await card.getByRole('button', { name: /Production/ }).click()
  await card
    .getByLabel('Your answer to: Which environments need it?')
    .fill('and the preview apps')
  await card.getByText('Which environments need it?').hover()
  await page.waitForTimeout(300)
  await page.screenshot({ path: 'e2e/output/question-card.png' })
  await send.click()

  expect(await answered(page)).toEqual([
    {
      id: 'q1',
      answers: {
        'Which database should the cache use?': 'Redis',
        'Which environments need it?':
          'Staging, Production, and the preview apps',
      },
    },
  ])
  await expect(card).toContainText('Answered')
  await expect(card).toContainText('Staging, Production, and the preview apps')
  await expect(card.getByRole('textbox')).toHaveCount(0)
})

test('answers in free text alone with Enter', async ({ page }) => {
  const card = await openQuestion(page, QUESTIONS.slice(0, 1))
  const box = card.getByRole('textbox')
  await box.fill('Neither, use SQLite')
  await box.press('Enter')

  expect(await answered(page)).toEqual([
    {
      id: 'q1',
      answers: {
        'Which database should the cache use?': 'Neither, use SQLite',
      },
    },
  ])
})

test('skips a question without answering it', async ({ page }) => {
  const card = await openQuestion(page)
  await card.getByRole('button', { name: 'Skip' }).click()

  await expect(card).toContainText('Skipped')
  expect(await answered(page)).toEqual([])
})
