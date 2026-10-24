import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const PLAN = `# Plan: Add retries to the Stripe webhook

## Context
Webhook deliveries fail when Stripe times out.

## Steps
1. Wrap the handler call in a retry loop with exponential backoff.
2. Add a test that fails twice, then succeeds.

## Verification
Run \`pnpm test\`.`

/** Selects `text` inside the rendered plan, the way dragging over it would. */
async function selectInPlan(page: Page, text: string) {
  await page.locator('[data-od-id="plan-markdown"]').evaluate((root, text) => {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
    while (walker.nextNode()) {
      const node = walker.currentNode as Text
      const at = node.data.indexOf(text)
      if (at < 0) continue
      const range = document.createRange()
      range.setStart(node, at)
      range.setEnd(node, at + text.length)
      const selection = window.getSelection()!
      selection.removeAllRanges()
      selection.addRange(range)
      return
    }
    throw new Error(`"${text}" isn't in the plan`)
  }, text)
}

async function openWorkspace(page: Page) {
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
}

test('plan: comment on highlighted text and send the comments back', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.addInitScript((plan) => {
    ;(window as { __HARNESS_PLAN__?: string }).__HARNESS_PLAN__ = plan
  }, PLAN)
  await openWorkspace(page)

  // The thread's approval card leads to the Plan tab, which says it's waiting.
  const planTab = page.locator('#thread-tab-plan')
  await expect(planTab).toHaveAccessibleName('Plan, 1 plan waiting for review')
  const row = page.locator('[data-od-id="approval-plan-1"]')
  await expect(row).toContainText(
    'Plan ready for review · Add retries to the Stripe webhook',
  )
  // The plan's row replaces the raw ExitPlanMode step, and the agent waits instead of working.
  await expect(page.getByText(/exit ?plan ?mode/i)).toHaveCount(0)
  await expect(page.locator('#thread-tab-th-lead')).toHaveAccessibleName(
    /Plan ready for review/,
  )
  await expect(page.locator('.agent-working-text')).toHaveCount(0)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/plan-thread-row.png' })
  await row.getByRole('button', { name: 'Review plan' }).click()
  const panel = page.locator('#plan-panel')
  await expect(
    panel.getByRole('heading', { name: 'Review the plan' }),
  ).toBeVisible()
  await expect(
    panel.getByRole('heading', {
      name: 'Plan: Add retries to the Stripe webhook',
    }),
  ).toBeVisible()
  await expect(panel.getByRole('heading', { name: 'Steps' })).toBeVisible()

  // Highlight a step and comment on it.
  await selectInPlan(page, 'exponential backoff')
  await panel.locator('[data-od-id="plan-comment-start"]').click()
  const form = panel.locator('[data-od-id="plan-comment-form"]')
  await form
    .getByRole('textbox', { name: 'Comment on the highlighted text' })
    .fill('Cap it at 3 attempts')
  await expect(form.getByRole('button', { name: /Add comment/ })).toBeEnabled()
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/plan-comment-form.png' })
  await form.getByRole('textbox').press('Meta+Enter')
  await expect(form).toHaveCount(0)

  // A second comment, saved with the button.
  await selectInPlan(page, 'fails twice, then succeeds')
  await panel.locator('[data-od-id="plan-comment-start"]').click()
  await form.getByRole('textbox').fill('Also cover the give-up path')
  await form.getByRole('button', { name: /Add comment/ }).click()

  const comments = panel.locator('[data-od-id="plan-comment"]')
  await expect(comments).toHaveCount(2)
  await expect(comments.first()).toContainText('exponential backoff')
  await expect(comments.first()).toContainText('Cap it at 3 attempts')

  // Approving waits until the comments are sent or deleted.
  await expect(panel.locator('[data-od-id="plan-approve"]')).toBeDisabled()
  await page.screenshot({ path: 'e2e/output/plan-comments.png' })

  await panel.getByRole('button', { name: 'Send 2 comments' }).click()
  await expect(
    panel.getByRole('heading', { name: 'Feedback sent' }),
  ).toBeVisible()
  await expect(panel.locator('[data-od-id="plan-status"]')).toContainText(
    'is revising the plan',
  )
  const resolved = await page.evaluate(
    () => (window as { __HARNESS_RESOLVED__?: unknown[] }).__HARNESS_RESOLVED__,
  )
  expect(resolved).toEqual([
    {
      id: 'plan-1',
      approved: false,
      denyReason: [
        "I reviewed your plan. Revise it with this feedback, then present the updated plan. Don't start editing yet.",
        '',
        '> exponential backoff',
        'Cap it at 3 attempts',
        '',
        '> fails twice, then succeeds',
        'Also cover the give-up path',
      ].join('\n'),
    },
  ])
  await expect(planTab).toHaveAccessibleName('Plan')

  // Back in the thread, the row says the plan went back, next to a slim feedback step.
  await page.locator('#thread-tab-th-lead').click()
  await expect(row).toContainText('Plan sent back')
  await expect(row.getByRole('button', { name: 'Open plan' })).toBeVisible()
  expect(errors).toEqual([])
})

test('plan: a revised plan opens the Plan tab, and approving returns to the thread', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await openWorkspace(page)
  await expect(page.locator('#thread-tab-plan')).toHaveCount(0)

  // The agent hands a plan over while the user is on the thread.
  await page.evaluate((plan) => {
    ;(window as { __HARNESS_ADD_PLAN__?: (id: string, plan: string) => void })
      .__HARNESS_ADD_PLAN__!('plan-2', plan)
  }, PLAN)
  const panel = page.locator('#plan-panel')
  await expect(panel).toBeVisible()
  await expect(page.locator('#thread-tab-plan')).toHaveAttribute(
    'aria-selected',
    'true',
  )

  await panel.locator('[data-od-id="plan-approve"]').click()
  await expect(page.locator('#thread-panel')).toBeVisible()
  const resolved = await page.evaluate(
    () => (window as { __HARNESS_RESOLVED__?: unknown[] }).__HARNESS_RESOLVED__,
  )
  expect(resolved).toEqual([{ id: 'plan-2', approved: true, denyReason: null }])

  // The approved plan stays readable in its tab.
  await page.locator('#thread-tab-plan').click()
  await expect(
    panel.getByRole('heading', { name: 'Plan approved' }),
  ).toBeVisible()
  await expect(panel.locator('[data-od-id="plan-approve"]')).toHaveCount(0)
  expect(errors).toEqual([])
})

test('plan: the composer switches a Claude thread to plan mode', async ({
  page,
}) => {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_LEAD_ENGINE__?: string }).__HARNESS_LEAD_ENGINE__ =
      'claude'
  })
  await openWorkspace(page)
  const toggle = page.locator('[data-od-id="composer-plan-mode"]')
  await expect(toggle).toHaveAttribute('aria-pressed', 'false')
  await toggle.click()
  await expect(toggle).toHaveAttribute('aria-pressed', 'true')
  await expect(page.locator('#composer-input')).toHaveAttribute(
    'placeholder',
    /to plan something/,
  )
})

test('plan: a message sent while a plan waits goes back as feedback on it', async ({
  page,
}) => {
  await page.addInitScript((plan) => {
    ;(window as { __HARNESS_PLAN__?: string }).__HARNESS_PLAN__ = plan
  }, PLAN)
  await openWorkspace(page)
  const input = page.locator('#composer-input')
  await expect(input).toHaveAttribute('placeholder', /Reply to .*’s plan/)
  await input.fill('Use a queue instead of retrying inline')
  await input.press('Enter')
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as { __HARNESS_RESOLVED__?: unknown[] }).__HARNESS_RESOLVED__,
      ),
    )
    .toEqual([
      {
        id: 'plan-1',
        approved: false,
        denyReason:
          "I reviewed your plan. Revise it with this feedback, then present the updated plan. Don't start editing yet.\n\nUse a queue instead of retrying inline",
      },
    ])
  // Feedback keeps the agent planning, so the composer is still in plan mode.
  await expect(input).toHaveAttribute('placeholder', /to plan something/)
})

test('plan: a waiting plan adds no approval pill to the agent tab or sidebar', async ({
  page,
}) => {
  await page.addInitScript((plan) => {
    ;(window as { __HARNESS_PLAN__?: string }).__HARNESS_PLAN__ = plan
  }, PLAN)
  await openWorkspace(page)
  await expect(page.locator('#thread-tab-th-lead')).toHaveText('Holly')
  await expect(page.locator('#thread-tab-th-lead')).not.toHaveAccessibleName(
    /approval/,
  )
  const workspaceRow = page
    .getByRole('navigation', { name: 'Harness' })
    .getByRole('button', { name: /^OAuth login, / })
  await expect(workspaceRow).toBeVisible()
  await expect(workspaceRow).not.toHaveAccessibleName(/approval/)
})

test('plan: turning plan mode off drops the waiting plan and closes its tab', async ({
  page,
}) => {
  await page.addInitScript((plan) => {
    ;(window as { __HARNESS_PLAN__?: string }).__HARNESS_PLAN__ = plan
  }, PLAN)
  await openWorkspace(page)
  const toggle = page.locator('[data-od-id="composer-plan-mode"]')
  await expect(toggle).toHaveAttribute('aria-pressed', 'true')
  await toggle.click()
  await expect(toggle).toHaveAttribute('aria-pressed', 'false')
  await expect(page.locator('#thread-tab-plan')).toHaveCount(0)
  const row = page.locator('[data-od-id="approval-plan-1"]')
  await expect(row).toContainText('Left plan mode before reviewing')
  await expect(row.getByRole('button')).toHaveCount(0)
  await expect(page.locator('#composer-input')).toHaveAttribute(
    'placeholder',
    /^Message /,
  )
})

test('plan: closing the Plan tab leaves plan mode', async ({ page }) => {
  await page.addInitScript((plan) => {
    ;(window as { __HARNESS_PLAN__?: string }).__HARNESS_PLAN__ = plan
  }, PLAN)
  await openWorkspace(page)
  await page.locator('#thread-tab-plan').click()
  await page.locator('[data-od-id="thread-close-plan"]').click()
  await expect(page.locator('#thread-tab-plan')).toHaveCount(0)
  await expect(page.locator('#thread-panel')).toBeVisible()
  await expect(
    page.locator('[data-od-id="composer-plan-mode"]'),
  ).toHaveAttribute('aria-pressed', 'false')
  await expect(page.locator('[data-od-id="approval-plan-1"]')).toContainText(
    'Left plan mode before reviewing',
  )
})
