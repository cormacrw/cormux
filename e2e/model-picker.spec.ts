import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('the composer picks a model per thread', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  mkdirSync('e2e/output', { recursive: true })
  await page.goto('/')

  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const picker = page.getByRole('button', { name: 'Model: Default' })
  await expect(picker).toBeVisible()
  await picker.click()
  const menu = page.getByRole('menu')
  await expect(menu.getByRole('menuitemradio')).toHaveText([
    'Default',
    'Fable',
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

test('opening the palette closes an open model picker', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.getByRole('button', { name: 'Model: Default' }).click()
  const menu = page.getByRole('menu')
  await expect(menu).toBeVisible()

  await page.keyboard.press('ControlOrMeta+k')
  await expect(page.getByRole('dialog')).toBeVisible()
  await expect(menu).toHaveCount(0)
})

test('the composer picks an effort per thread', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()

  await page.getByRole('button', { name: 'Effort: Default' }).click()
  const menu = page.getByRole('menu')
  await expect(menu.getByRole('menuitemradio')).toHaveText([
    'Default',
    'Low',
    'Medium',
    'High',
    'Extra high',
    'Max',
  ])
  await menu.getByRole('menuitemradio', { name: 'High', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Effort: High' })).toBeVisible()
})

test('⌘⇧M and ⌘⇧E pick with the arrow keys and ↵', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const composer = page.locator('#composer-input')
  await expect(composer).toBeFocused()

  await page.keyboard.press('ControlOrMeta+Shift+M')
  // A closing menu lingers through its animation.
  const menu = page.locator('[role="menu"][data-state="open"]')
  await expect(
    menu.getByRole('menuitemradio', { name: 'Default' }),
  ).toBeFocused()
  await page.keyboard.press('ArrowDown')
  await page.keyboard.press('ArrowDown')
  await expect(menu.getByRole('menuitemradio', { name: 'Opus' })).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(menu).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Model: Opus' })).toBeVisible()
  await expect(composer).toBeFocused()

  // The next time, the arrow keys start from the pick.
  await page.keyboard.press('ControlOrMeta+Shift+M')
  await expect(menu.getByRole('menuitemradio', { name: 'Opus' })).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(menu).toHaveCount(0)
  await expect(composer).toBeFocused()

  await page.keyboard.press('ControlOrMeta+Shift+E')
  await expect(
    menu.getByRole('menuitemradio', { name: 'Default' }),
  ).toBeFocused()
  await page.keyboard.press('ArrowUp')
  await expect(menu.getByRole('menuitemradio', { name: 'Max' })).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('button', { name: 'Effort: Max' })).toBeVisible()
  await expect(composer).toBeFocused()

  // One picker gives way to the other, and the shortcut closes its own.
  await page.keyboard.press('ControlOrMeta+Shift+M')
  await expect(menu).toContainText('Model')
  await page.keyboard.press('ControlOrMeta+Shift+E')
  await expect(menu).toHaveCount(1)
  await expect(menu).toContainText('Effort')
  await expect(menu.getByRole('menuitemradio', { name: 'Max' })).toBeFocused()
  await page.keyboard.press('ControlOrMeta+Shift+E')
  await expect(menu).toHaveCount(0)
  await expect(composer).toBeFocused()
})

test('the pickers stay shut while Git covers the composer', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.keyboard.press('ControlOrMeta+G')
  await expect(
    page
      .locator('[data-od-id="composer"]')
      .locator('xpath=ancestor::*[@inert]'),
  ).not.toHaveCount(0)
  await page.keyboard.press('ControlOrMeta+Shift+M')
  await expect(page.getByRole('menu')).toHaveCount(0)
})
