import { expect, test } from '@playwright/test'
import { setScenario } from './harness'

test('the palette opens a workspace’s retired thread', async ({ page }) => {
  await setScenario(page, { retiredThread: true })
  await page.goto('/')
  await page.getByRole('button', { name: /Search or run a command/ }).click()
  const dialog = page.getByRole('dialog')
  await page.keyboard.type('Open Reviewer')
  const option = dialog.getByText('Open Reviewer in OAuth login')
  await expect(option).toBeVisible()
  await option.click()
  await expect(dialog).toHaveCount(0)
  await expect(page.getByRole('heading', { name: 'OAuth login' })).toBeVisible()
})
