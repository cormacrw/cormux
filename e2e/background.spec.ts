import { expect, test, type Page } from '@playwright/test'

// These specs check the defaults, so start without the suite's still background.
test.use({ storageState: { cookies: [], origins: [] } })

async function openAppearance(page: Page) {
  await page.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('button', { name: 'Appearance', exact: true }).click()
  return page.locator('[data-od-id="settings-appearance-group"]')
}

test('background picker selects a scene and persists it', async ({ page }) => {
  await page.goto('/')
  const html = page.locator('html')
  await expect(html).toHaveAttribute('data-background', 'studio')

  const panel = await openAppearance(page)
  const picker = panel.getByRole('radiogroup', { name: 'Background' })
  const tiles = picker.getByRole('radio')
  await expect(tiles).toHaveText([
    'Studio',
    'Orbs',
    'Waves',
    'Crystals',
    'Particles',
    'Aurora',
    'Gradient',
  ])
  await expect(picker.getByRole('radio', { name: 'Studio' })).toBeChecked()

  await picker.getByRole('radio', { name: 'Aurora' }).click()
  await expect(picker.getByRole('radio', { name: 'Aurora' })).toBeChecked()
  await expect(html).toHaveAttribute('data-background', 'aurora')
  await page.screenshot({ path: 'e2e/output/appearance-background.png' })

  await page.reload()
  await expect(html).toHaveAttribute('data-background', 'aurora')
  await expect(
    (await openAppearance(page))
      .getByRole('radiogroup', { name: 'Background' })
      .getByRole('radio', { name: 'Aurora' }),
  ).toBeChecked()
})

test('animate background switch persists', async ({ page }) => {
  await page.goto('/')
  const panel = await openAppearance(page)
  const animate = panel.getByRole('switch', { name: 'Animate background' })
  await expect(animate).toBeChecked()
  await animate.click()
  await expect(animate).not.toBeChecked()
  expect(
    await page.evaluate(() =>
      localStorage.getItem('cormux-background-animate'),
    ),
  ).toBe('0')

  await page.reload()
  await expect(
    (await openAppearance(page)).getByRole('switch', {
      name: 'Animate background',
    }),
  ).not.toBeChecked()
})
