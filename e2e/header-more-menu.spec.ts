import { expect, test } from '@playwright/test'

test('a header too narrow for every button folds them into ⋯', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.setViewportSize({ width: 1100, height: 600 })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()

  await expect(
    page.getByRole('heading', { name: 'OAuth login', level: 1 }),
  ).toBeVisible()
  await expect(page.locator('[data-od-id="ws-terminal"]')).toHaveCount(0)
  await page.getByRole('button', { name: 'More workspace actions' }).click()
  const menu = page.getByRole('menu')
  await expect(menu.getByRole('menuitem', { name: /Run/ })).toBeVisible()
  await expect(menu.getByRole('menuitem', { name: /Restart/ })).toHaveCount(0)
  await expect(
    menu.getByRole('menuitem', { name: /Review changes/ }),
  ).toBeVisible()
  await page.keyboard.press('Escape')

  // Wide enough, the buttons are back, with Review labelled.
  await page.setViewportSize({ width: 1600, height: 600 })
  await expect(page.locator('[data-od-id="ws-terminal"]')).toBeVisible()
  await expect(page.locator('[data-od-id="ws-review"]')).toContainText('Review')
  await expect(page.locator('[data-od-id="ws-review"]')).toHaveAttribute(
    'aria-keyshortcuts',
    'Meta+I',
  )
  expect(errors).toEqual([])
})
