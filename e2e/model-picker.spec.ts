import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('the composer picks a model per thread when the engine lists some', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  mkdirSync('e2e/output', { recursive: true })
  await page.goto('/')

  // The fixture Lead runs Cursor, which lists no models until its session starts.
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await expect(page.locator('#composer')).toBeVisible()
  await expect(page.locator('[data-od-id="composer-model"]')).toHaveCount(0)

  await page.goto('/')
  await page
    .getByRole('button', {
      name: 'Why the webhook signature fails, my-app, Idle',
    })
    .click()
  const picker = page.getByRole('button', { name: 'Model: Default' })
  await expect(picker).toBeVisible()
  await picker.click()
  const menu = page.getByRole('menu')
  await expect(menu.getByRole('menuitemradio')).toHaveText([
    'Default',
    'Opus',
    'Sonnet',
    'Haiku',
  ])
  await expect(
    menu.getByRole('menuitemradio', { name: 'Default' }),
  ).toHaveAttribute('aria-checked', 'true')
  await page.waitForTimeout(400)
  await page.screenshot({ path: 'e2e/output/model-picker.png' })

  await menu.getByRole('menuitemradio', { name: 'Opus' }).click()
  await expect(menu).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Model: Opus' })).toBeVisible()

  // The pick is saved, so a fresh load of the menu still shows it.
  await page.getByRole('button', { name: 'Model: Opus' }).click()
  await expect(
    page.getByRole('menuitemradio', { name: 'Opus' }),
  ).toHaveAttribute('aria-checked', 'true')
  await page.keyboard.press('Escape')

  expect(errors).toEqual([])
})
