import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('right-click snoozes a thread into its own section, and back out', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  const threads = page.locator('[data-od-id="thread-list"]')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  const toggle = page.locator('[data-od-id="snoozed-toggle"]')
  await expect(threads.locator(row)).toBeVisible()
  await expect(toggle).toHaveCount(0)

  await row.click({ button: 'right' })
  const menu = page.locator('[data-od-id="thread-row-menu"]')
  await expect(menu).toBeVisible()
  await expect(menu.getByRole('menuitem')).toHaveText([
    /^1 hour/,
    /^3 hours/,
    /^Tomorrow/,
    /^Next week/,
    /Delete…$/,
  ])
  mkdirSync('e2e/output', { recursive: true })
  await page.waitForTimeout(300) // let the menu finish opening
  await page.screenshot({ path: 'e2e/output/snooze-menu.png' })
  await menu.getByRole('menuitem', { name: /^Tomorrow/ }).click()

  await expect(threads.locator(row)).toHaveCount(0)
  await expect(toggle).toContainText('1')
  await expect(toggle).toHaveAttribute('aria-expanded', 'false')
  await toggle.click()
  const snoozed = page.locator('[data-od-id="snoozed-list"]').locator(row)
  await expect(snoozed).toBeVisible()
  await expect(snoozed).toHaveAccessibleName(/snoozed until Tomorrow 9/)
  await page.waitForTimeout(400) // let the section finish opening
  await page.screenshot({ path: 'e2e/output/snoozed-section.png' })

  // Kept across a reload, since it's how you left the sidebar.
  await page.reload()
  await expect(
    page.locator('[data-od-id="snoozed-list"]').locator(row),
  ).toBeVisible()

  await page
    .locator('[data-od-id="snoozed-list"]')
    .locator(row)
    .click({ button: 'right' })
  await menu.getByRole('menuitem', { name: 'Unsnooze' }).click()
  await expect(threads.locator(row)).toBeVisible()
  await expect(toggle).toHaveCount(0)
})

test('a snooze ends on its own when its time is up', async ({ page }) => {
  await page.clock.install()
  await page.goto('/')
  const threads = page.locator('[data-od-id="thread-list"]')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await row.click({ button: 'right' })
  await page
    .locator('[data-od-id="thread-row-menu"]')
    .getByRole('menuitem', { name: /^1 hour/ })
    .click()
  await expect(threads.locator(row)).toHaveCount(0)

  await page.clock.runFor('01:00:05')
  await expect(threads.locator(row)).toBeVisible()
  await expect(page.locator('[data-od-id="snoozed-toggle"]')).toHaveCount(0)
})

test('right-click Delete asks first, then removes the thread', async ({
  page,
}) => {
  await page.goto('/')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await row.click({ button: 'right' })
  await page.locator('[data-od-id="thread-row-delete"]').click()
  const teardown = page.getByRole('alertdialog')
  await expect(teardown).toBeVisible()
  await teardown.getByRole('button', { name: 'Teardown' }).click()
  await expect(teardown).toHaveCount(0)
  await expect(row).toHaveCount(0)

  // A read-only scratch gets its own delete confirmation.
  const scratch = page.getByRole('button', { name: /^Why the webhook/ })
  await scratch.click({ button: 'right' })
  await page.locator('[data-od-id="thread-row-delete"]').click()
  const confirm = page.getByRole('alertdialog')
  await expect(confirm).toBeVisible()
  await confirm.getByRole('button', { name: /delete/i }).click()
  await expect(scratch).toHaveCount(0)
})
