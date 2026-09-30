import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('Changes panel lets you compare against another branch', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.getByRole('tab', { name: /^Changes/ }).click()
  const panel = page.getByRole('tabpanel', { name: /^Changes/ })
  const picker = panel.getByRole('button', { name: /Compare against/ })
  await expect(picker).toContainText('Uncommitted')
  await picker.click()
  const menu = page.getByRole('menu', { name: 'Compare against' })
  await expect(menu.getByText('Uncommitted changes')).toBeVisible()
  await expect(menu.getByText('main', { exact: true })).toBeVisible()
  await page.waitForTimeout(400)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/changes-target.png' })
})

test('Changes panel shows a loading splash while the diff is fetched', async ({
  page,
}) => {
  await page.goto('/?slowDiff=1')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.getByRole('tab', { name: /^Changes/ }).click()
  const panel = page.getByRole('tabpanel', { name: /^Changes/ })
  // Files already on screen stay put during a refresh.
  await expect(
    panel.getByRole('region', { name: 'Proposed changes' }),
  ).toBeVisible()
  await expect(panel.getByRole('status')).toHaveCount(0)

  await panel.getByRole('button', { name: /Compare against/ }).click()
  await page
    .getByRole('menu', { name: 'Compare against' })
    .getByText('main', { exact: true })
    .click()
  const splash = panel.getByRole('status')
  await expect(splash).toHaveText('Loading changes since main…')
  await expect(
    panel.getByRole('button', { name: /Compare against/ }),
  ).toContainText('vs main')
  // The splash covers the list instead of unmounting it.
  await expect(
    panel.getByRole('region', { name: 'Proposed changes' }),
  ).toHaveCount(1)
  await page.waitForTimeout(400)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/changes-loading.png' })

  await expect(splash).toHaveCount(0)
  await expect(
    panel.getByRole('region', { name: 'Proposed changes' }),
  ).toBeVisible()
  await expect(
    panel.getByRole('button', { name: /Compare against/ }),
  ).toContainText('vs main')
})
