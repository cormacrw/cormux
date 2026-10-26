import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('New workspace picks the repo from an app menu, not a native select', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await expect(page.locator('#home-title')).toBeVisible()
  await page.keyboard.press('ControlOrMeta+n')
  const dialog = page.getByRole('dialog', { name: 'New workspace' })
  await expect(dialog).toBeVisible()
  await expect(dialog.locator('select')).toHaveCount(0)

  // Claude is the only engine, so there is nothing to pick.
  await expect(dialog.getByRole('button', { name: 'AI engine' })).toHaveCount(0)

  // The label still names the control, and Esc closes the menu, not the dialog.
  const repo = dialog.getByRole('button', { name: 'Repository' })
  await expect(repo).toHaveText('my-app')
  await repo.click()
  const menu = page.locator('[data-slot="select-content"]')
  await expect(menu).toBeVisible()
  await page.waitForTimeout(400)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/select-repo.png' })
  await page.keyboard.press('Escape')
  await expect(menu).toHaveCount(0)
  await expect(dialog).toBeVisible()
  expect(errors).toEqual([])
})
