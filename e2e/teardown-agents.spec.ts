import { expect, test } from '@playwright/test'
import { setScenario } from './harness'

test('deleting a workspace takes its agents out of the sidebar with it', async ({
  page,
}) => {
  await page.goto('/')
  const sidebar = page.getByRole('navigation', { name: 'Sidebar' })
  const row = sidebar.getByRole('button', {
    name: /^OAuth login,/,
  })
  await expect(row).toBeVisible()

  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.keyboard.press('ControlOrMeta+d')
  const teardown = page.getByRole('alertdialog')
  await teardown.getByRole('button', { name: 'Teardown' }).click()
  await expect(teardown).toHaveCount(0)

  await expect(row).toHaveCount(0)
})

test('a clean worktree is deleted without asking', async ({ page }) => {
  await setScenario(page, { cleanTeardown: 'ws-auth' })
  await page.goto('/')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await row.click()
  await page.keyboard.press('ControlOrMeta+d')

  await expect(row).toHaveCount(0)
  await expect(page.getByRole('alertdialog')).toHaveCount(0)
  await expect(page.getByRole('heading', { name: 'New thread' })).toBeVisible()
})
