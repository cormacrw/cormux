import { expect, test } from '@playwright/test'

test('deleting a workspace takes its agents out of the sidebar with it', async ({
  page,
}) => {
  await page.goto('/')
  const sidebar = page.getByRole('navigation', { name: 'Harness' })
  const agent = sidebar.getByRole('button', {
    name: /^Holly, .* in OAuth login/,
  })
  await expect(agent).toBeVisible()

  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.keyboard.press('ControlOrMeta+d')
  const teardown = page.getByRole('alertdialog')
  await teardown.getByRole('button', { name: 'Teardown' }).click()
  await expect(teardown).toHaveCount(0)

  await expect(
    sidebar.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }),
  ).toHaveCount(0)
  await expect(agent).toHaveCount(0)
})
