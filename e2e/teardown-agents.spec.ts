import { expect, test } from '@playwright/test'

test('deleting a workspace takes its agents out of the sidebar with it', async ({
  page,
}) => {
  await page.goto('/')
  const sidebar = page.getByRole('navigation', { name: 'Harness' })
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
