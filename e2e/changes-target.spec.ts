import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('Changes panel lets you compare against another branch', async ({ page }) => {
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
