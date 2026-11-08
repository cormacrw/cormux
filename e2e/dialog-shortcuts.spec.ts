import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'
import type { MenuCommand } from '../src/lib/ipc/bindings'

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

test('menu bar commands wait while a dialog is open, as their keys do', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.keyboard.press('ControlOrMeta+d')
  const teardown = page.getByRole('alertdialog')
  await expect(teardown).toBeVisible()
  const choose = (command: MenuCommand) =>
    page.evaluate((command) => window.__HARNESS__!.chooseMenu(command), command)

  await choose('newWorkspace')
  await choose('openSettings')
  await page.waitForTimeout(200)
  await expect(teardown).toBeVisible()
  // The open dialog hides the page from the accessibility tree, so look past that.
  const heading = (name: string) =>
    page.getByRole('heading', { name, exact: true, includeHidden: true })
  await expect(heading('OAuth login')).toBeVisible()
  await expect(heading('New thread')).toHaveCount(0)
  await expect(heading('Settings')).toHaveCount(0)

  await teardown.getByRole('button', { name: 'Cancel' }).click()
  await expect(teardown).toHaveCount(0)
  await choose('newWorkspace')
  await expect(page.getByRole('heading', { name: 'New thread' })).toBeVisible()
})
