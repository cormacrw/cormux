import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('New workspace picks the engine and repo from app menus, not native selects', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await page.getByRole('button', { name: 'New Workspace', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'New workspace' })
  await expect(dialog).toBeVisible()
  await expect(dialog.locator('select')).toHaveCount(0)

  // The label still names the control.
  const engine = dialog.getByRole('button', { name: 'AI engine' })
  await expect(engine).toHaveText(/Cursor CLI/)
  await engine.click()
  const menu = page.locator('[data-slot="select-content"]')
  await expect(menu.getByRole('option').first()).toBeVisible()
  await page.waitForTimeout(400)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/select-engine.png' })
  await page.getByRole('option', { name: /^Claude Code/ }).click()
  await expect(engine).toHaveText(/Claude Code/)
  await expect(menu).toHaveCount(0)

  // Esc closes the menu, not the dialog.
  const repo = dialog.getByRole('button', { name: 'Repository' })
  await expect(repo).toHaveText('my-app')
  await repo.click()
  await expect(menu).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(menu).toHaveCount(0)
  await expect(dialog).toBeVisible()
  expect(errors).toEqual([])
})
