import { expect, test } from '@playwright/test'

test('the header keeps actions in ⋯ at any width, beside an icon Run button', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.addInitScript(() => {
    ;(window as { __HARNESS_RUN_COMMAND__?: string }).__HARNESS_RUN_COMMAND__ =
      'pnpm dev'
  })
  await page.setViewportSize({ width: 1600, height: 700 })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle' }).click()
  await expect(
    page.getByRole('heading', { name: 'OAuth login', level: 1 }),
  ).toBeVisible()

  // Even wide, only Run and ⋯ show.
  await expect(page.locator('[data-od-id="ws-terminal"]')).toHaveCount(0)
  await expect(page.locator('[data-od-id="ws-review"]')).toHaveCount(0)
  const run = page.getByRole('button', { name: 'Run app' })
  await expect(run).toBeVisible()
  await expect(run).toHaveText('')

  await page.getByRole('button', { name: 'More workspace actions' }).click()
  const menu = page.getByRole('menu')
  await expect(
    menu.getByRole('menuitem', { name: /Open in Terminal/ }),
  ).toBeVisible()
  await expect(
    menu.getByRole('menuitem', { name: /Review changes/ }),
  ).toBeVisible()
  await expect(
    menu.getByRole('menuitem', { name: /Delete workspace/ }),
  ).toBeVisible()
  await page.keyboard.press('Escape')

  // Running, Run becomes Stop and Restart side by side.
  await run.click()
  await expect(page.getByRole('button', { name: 'Stop app' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Restart app' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Run app' })).toHaveCount(0)
  expect(errors).toEqual([])
})
