import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('dialogs show Esc and ⌘↵, and ⌘↵ runs the action', async ({ page }) => {
  mkdirSync('e2e/output', { recursive: true })
  await page.goto('/')

  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.keyboard.press('ControlOrMeta+d')
  const teardown = page.getByRole('alertdialog')
  await expect(teardown.getByRole('button', { name: 'Teardown' })).toBeEnabled()
  await expect(teardown.locator('kbd')).toHaveCount(2)
  await page.waitForTimeout(300) // let the open animation finish
  await page.screenshot({ path: 'e2e/output/dialog-shortcuts-teardown.png' })
  await page.keyboard.press('ControlOrMeta+Enter')
  await expect(teardown).toHaveCount(0)
  await expect(page.getByRole('heading', { name: 'New thread' })).toBeVisible()
})
