import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('comment on a diff line and send it to the agent', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.getByRole('tab', { name: /^Git/ }).click()
  const panel = page.getByRole('tabpanel', { name: /^Git/ })
  const diff = panel.getByRole('region', { name: 'Proposed changes' })
  await expect(diff.getByText('provider?: string').first()).toBeVisible()
  await expect(panel.getByRole('button', { name: 'Approve file' })).toHaveCount(
    0,
  )

  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/changes-diff.png' })

  // Both files start expanded, can fold independently, and the list scrolls.
  const files = panel.getByRole('list', { name: 'Changed files' })
  const providers = files.getByRole('button', { name: /providers\.ts/ })
  await expect(providers).toHaveAttribute('aria-expanded', 'true')
  const scroll = await diff.evaluate((el) => {
    el.scrollTop = 400
    return { top: el.scrollTop, overflow: el.scrollHeight > el.clientHeight }
  })
  expect(scroll.overflow).toBe(true)
  expect(scroll.top).toBeGreaterThan(0)
  await page.screenshot({ path: 'e2e/output/changes-scrolled.png' })
  await diff.evaluate((el) => (el.scrollTop = 0))
  await providers.click()
  await expect(providers).toHaveAttribute('aria-expanded', 'false')
  await expect(diff.getByText("provider60 = 'p60'")).toHaveCount(0)
  await expect(
    files.getByRole('button', { name: /session\.ts/ }),
  ).toHaveAttribute('aria-expanded', 'true')
  await panel.getByRole('button', { name: 'Collapse all' }).click()
  await expect(diff.getByText('provider?: string')).toHaveCount(0)
  await panel.getByRole('button', { name: 'Expand all' }).click()
  await expect(providers).toHaveAttribute('aria-expanded', 'true')

  const line = diff
    .locator('tr', { hasText: "if (provider === 'github')" })
    .first()
  await line.hover()
  await line.locator('.diff-add-widget').first().click()
  const box = panel.getByRole('textbox', { name: /Comment on line/ })
  await box.fill('Use a map of providers instead of an if')
  await page.screenshot({ path: 'e2e/output/changes-comment-compose.png' })
  await panel.getByRole('button', { name: 'Add comment' }).click()
  await expect(
    diff.getByText('Use a map of providers instead of an if'),
  ).toBeVisible()
  await expect(
    files.getByRole('button', { name: /session\.ts/ }),
  ).toContainText('1')

  // The counter sits on the stack level the comment was written on.
  const stack = page.getByRole('navigation', { name: 'Stack' })
  await expect(
    stack.locator('[data-stack-uncommitted]').getByText('1 comment'),
  ).toHaveCount(1)
  await expect(stack.getByText(/\d+ comments?$/)).toHaveCount(1)

  await page.waitForTimeout(300)
  await page.screenshot({ path: 'e2e/output/changes-comment.png' })

  await panel.getByRole('button', { name: 'Send 1 comment to Lead' }).click()
  await expect(page.locator('#timeline')).toContainText('src/auth/session.ts:4')
  await expect(page.locator('#timeline')).toContainText(
    'Use a map of providers instead of an if',
  )
  await expect(page.getByRole('tab', { name: /^Lead/ })).toHaveAttribute(
    'aria-selected',
    'true',
  )
  await page.getByRole('tab', { name: /^Git/ }).click()
  await expect(
    panel.getByRole('button', { name: /Send \d+ comment/ }),
  ).toHaveCount(0)
  expect(errors).toEqual([])
})
