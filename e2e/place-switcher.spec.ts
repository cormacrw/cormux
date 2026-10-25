import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const heading = (page: Page) => page.getByRole('heading', { level: 1 })

async function visitHomebaseWorkspaceWeek(page: Page) {
  await page.goto('/')
  await expect(heading(page)).toHaveText('Homebase')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  await expect(heading(page)).toHaveText('OAuth login')
  await page
    .getByRole('navigation', { name: 'Harness' })
    .getByRole('button', { name: /^Week/ })
    .click()
  await expect(heading(page)).toHaveText('This week')
}

test('a quick Ctrl+Tab goes back to the previous place, and again flips back', async ({
  page,
}) => {
  await visitHomebaseWorkspaceWeek(page)
  await page.keyboard.press('Control+Tab')
  await expect(heading(page)).toHaveText('OAuth login')
  await page.keyboard.press('Control+Tab')
  await expect(heading(page)).toHaveText('This week')
  // Too quick to show the list.
  await expect(page.locator('[data-od-id="place-switcher"]')).toHaveCount(0)
})

test('holding Ctrl lists places most recent first; letting go switches', async ({
  page,
}) => {
  await visitHomebaseWorkspaceWeek(page)
  await page.keyboard.down('Control')
  await page.keyboard.press('Tab')
  const switcher = page.locator('[data-od-id="place-switcher"]')
  await expect(switcher).toBeVisible()
  await expect(switcher.getByRole('option')).toHaveText([
    /^\s*Week$/,
    /^\s*OAuth login/,
    /^\s*Homebase$/,
  ])
  await expect(switcher.getByRole('option', { selected: true })).toHaveText(
    /^\s*OAuth login/,
  )
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/place-switcher.png' })

  await page.keyboard.press('Tab')
  await expect(switcher.getByRole('option', { selected: true })).toHaveText(
    /^\s*Homebase$/,
  )
  await page.keyboard.up('Control')
  await expect(switcher).toHaveCount(0)
  await expect(heading(page)).toHaveText('Homebase')

  // Homebase is now the newest, so the order follows.
  await page.keyboard.down('Control')
  await page.keyboard.press('Tab')
  await expect(switcher.getByRole('option')).toHaveText([
    /^\s*Homebase$/,
    /^\s*Week$/,
    /^\s*OAuth login/,
  ])
  // Esc closes it without going anywhere.
  await page.keyboard.press('Escape')
  await page.keyboard.up('Control')
  await expect(switcher).toHaveCount(0)
  await expect(heading(page)).toHaveText('Homebase')
})
