import { expect, test } from '@playwright/test'

test('Homebase shows today’s tasks; clicking one opens the task flyout', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  // The harness's sprint plans one task for today.
  await page.addInitScript(() => {
    ;(window as { __HARNESS_CLICKUP__?: boolean }).__HARNESS_CLICKUP__ = true
  })
  await page.goto('/')
  const today = page.locator('[data-od-id="home-today"]')
  await expect(today.getByRole('heading', { name: /Today/ })).toBeVisible()
  const card = today.locator('[data-week-item]').first()
  await expect(card).toBeVisible()
  await expect(page.locator('[data-od-id="home-pinned-todos"]')).toHaveCount(0)

  const flyout = page.locator('[data-od-id="task-flyout"]')
  await expect(flyout).toHaveAttribute('data-open', 'false')
  await card.locator('[data-week-card]').click()
  await expect(flyout).toHaveAttribute('data-open', 'true')
  await expect(flyout.getByRole('heading').first()).toBeVisible()

  // A click elsewhere in the app closes it.
  await page.getByRole('heading', { name: 'Homebase', level: 1 }).click()
  await expect(flyout).toHaveAttribute('data-open', 'false')

  // No TODOs page any more.
  await expect(
    page
      .getByRole('navigation', { name: 'Harness' })
      .getByRole('button', { name: /^TODOs/ }),
  ).toHaveCount(0)
  expect(errors).toEqual([])
})
