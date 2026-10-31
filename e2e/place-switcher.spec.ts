import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const SPRINT_TITLE = /Sprint/
const heading = (page: Page) => page.getByRole('heading', { level: 1 })

async function visitNewThreadWorkspaceSprint(page: Page) {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_CLICKUP__?: boolean }).__HARNESS_CLICKUP__ = true
  })
  await page.goto('/')
  await expect(heading(page)).toHaveText('New thread')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(heading(page)).toHaveText('OAuth login')
  await page
    .getByRole('navigation', { name: 'Harness' })
    .getByRole('button', { name: /^Sprint/ })
    .click()
  await expect(heading(page)).toHaveText(SPRINT_TITLE)
}

test('a quick Ctrl+Tab goes back to the previous place, and again flips back', async ({
  page,
}) => {
  await visitNewThreadWorkspaceSprint(page)
  await page.keyboard.press('Control+Tab')
  await expect(heading(page)).toHaveText('OAuth login')
  await page.keyboard.press('Control+Tab')
  await expect(heading(page)).toHaveText(SPRINT_TITLE)
  // Too quick to show the list.
  await expect(page.locator('[data-od-id="place-switcher"]')).toHaveCount(0)
})

test('holding Ctrl lists places most recent first; letting go switches', async ({
  page,
}) => {
  await visitNewThreadWorkspaceSprint(page)
  await page.keyboard.down('Control')
  await page.keyboard.press('Tab')
  const switcher = page.locator('[data-od-id="place-switcher"]')
  await expect(switcher).toBeVisible()
  await expect(switcher.getByRole('option')).toHaveText([
    /^\s*Sprint$/,
    /^\s*OAuth login/,
    /^\s*New thread$/,
  ])
  await expect(switcher.getByRole('option', { selected: true })).toHaveText(
    /^\s*OAuth login/,
  )
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/place-switcher.png' })

  await page.keyboard.press('Tab')
  await expect(switcher.getByRole('option', { selected: true })).toHaveText(
    /^\s*New thread$/,
  )
  await page.keyboard.up('Control')
  await expect(switcher).toHaveCount(0)
  await expect(heading(page)).toHaveText('New thread')

  // New thread is now the newest, so the order follows.
  await page.keyboard.down('Control')
  await page.keyboard.press('Tab')
  await expect(switcher.getByRole('option')).toHaveText([
    /^\s*New thread$/,
    /^\s*Sprint$/,
    /^\s*OAuth login/,
  ])
  // Esc closes it without going anywhere.
  await page.keyboard.press('Escape')
  await page.keyboard.up('Control')
  await expect(switcher).toHaveCount(0)
  await expect(heading(page)).toHaveText('New thread')
})
