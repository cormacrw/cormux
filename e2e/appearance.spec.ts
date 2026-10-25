import { expect, test, type Page } from '@playwright/test'

async function openAppearance(page: Page) {
  await page.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('button', { name: 'Appearance', exact: true }).click()
  return page.locator('[data-od-id="settings-appearance-group"]')
}

test('Mode setting overrides the system theme and persists', async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  const html = page.locator('html')
  await expect(html).toHaveClass(/\bdark\b/)

  const theme = (await openAppearance(page)).getByRole('group', {
    name: 'Mode',
  })
  await expect(theme.getByRole('radio', { name: 'System' })).toBeChecked()
  await theme.getByRole('radio', { name: 'Light' }).click()
  await expect(html).not.toHaveClass(/\bdark\b/)

  // index.html applies the saved choice before first paint.
  await page.reload()
  await expect(html).not.toHaveClass(/\bdark\b/)

  await (
    await openAppearance(page)
  )
    .getByRole('group', { name: 'Mode' })
    .getByRole('radio', { name: 'System' })
    .click()
  await expect(html).toHaveClass(/\bdark\b/)
})

function primary(page: Page) {
  return page.evaluate(() =>
    getComputedStyle(document.documentElement)
      .getPropertyValue('--primary')
      .trim(),
  )
}

test('colour theme and font sizes apply and persist', async ({ page }) => {
  await page.goto('/')
  const html = page.locator('html')
  const mono = page.locator('.font-mono').first()
  await expect(mono).toHaveCSS('font-family', /JetBrains Mono Variable/)
  await expect
    .poll(() =>
      page.evaluate(() =>
        document.fonts.check("16px 'JetBrains Mono Variable'"),
      ),
    )
    .toBe(true)
  // Blue is the default theme.
  await expect(html).toHaveAttribute('data-palette', 'blue')
  const blue = await primary(page)

  const panel = await openAppearance(page)
  const themes = panel.getByRole('radiogroup', { name: 'Theme' })
  await expect(themes.getByRole('radio', { name: 'Blue' })).toBeChecked()
  await themes.getByRole('radio', { name: 'Violet' }).click()
  await expect(html).toHaveAttribute('data-palette', 'violet')
  await expect.poll(() => primary(page)).not.toBe(blue)
  const violet = await primary(page)

  await themes.getByRole('radio', { name: 'Neutral' }).click()
  await expect(html).not.toHaveAttribute('data-palette', /.*/)
  await themes.getByRole('radio', { name: 'Violet' }).click()

  await panel.getByLabel('UI font size').click()
  await page.getByRole('option', { name: '18px' }).click()
  await expect(html).toHaveCSS('font-size', '18px')

  await panel.getByLabel('Code font size').click()
  await page.getByRole('option', { name: '14px' }).click()

  await panel.getByLabel('Code font', { exact: true }).click()
  const geist = page.getByRole('option', { name: 'Geist Mono' })
  await expect(geist).toBeVisible()
  await page.screenshot({ path: 'e2e/output/appearance-code-font.png' })
  await geist.click()
  await expect(mono).toHaveCSS('font-family', /Geist Mono Variable/)

  await page.reload()
  await expect(html).toHaveAttribute('data-palette', 'violet')
  await expect.poll(() => primary(page)).toBe(violet)
  await expect(html).toHaveCSS('font-size', '18px')
  expect(
    await html.evaluate((el) =>
      getComputedStyle(el).getPropertyValue('--code-font-size').trim(),
    ),
  ).toBe('14px')
  await expect(page.locator('.font-mono').first()).toHaveCSS(
    'font-family',
    /Geist Mono Variable/,
  )
})

test('the wordmark shows behind the glass and can be turned off', async ({
  page,
}) => {
  await page.goto('/')
  const wordmark = page.locator('[data-od-id="wordmark"]')
  await expect(wordmark).toBeAttached()
  // Faint, and never in the way of clicks.
  await expect(page.locator('[data-od-id="backdrop"]')).toHaveCSS(
    'pointer-events',
    'none',
  )

  const toggle = (await openAppearance(page)).getByRole('switch', {
    name: 'Show wordmark',
  })
  await expect(toggle).toBeChecked()
  await toggle.click()
  await expect(wordmark).not.toBeAttached()
  await page.reload()
  await expect(wordmark).not.toBeAttached()
})
