import { expect, test, type Page } from '@playwright/test'

// These specs check the defaults, so start without the suite's still background.
test.use({ storageState: { cookies: [], origins: [] } })

async function openAppearance(page: Page) {
  await page.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('button', { name: 'Appearance', exact: true }).click()
  return page.locator('[data-od-id="settings-appearance-group"]')
}

test('Theme setting overrides the system theme and persists', async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  const html = page.locator('html')
  await expect(html).toHaveClass(/\bdark\b/)

  const theme = (await openAppearance(page)).getByRole('group', {
    name: 'Theme',
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
    .getByRole('group', { name: 'Theme' })
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

test('primary color and font sizes apply and persist', async ({ page }) => {
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
  // Blue is the default accent.
  await expect(html).toHaveAttribute('data-accent', 'blue')
  const blue = await primary(page)

  const panel = await openAppearance(page)
  const colors = panel.getByRole('radiogroup', { name: 'Primary color' })
  await expect(colors.getByRole('radio', { name: 'Blue' })).toBeChecked()
  await colors.getByRole('radio', { name: 'Violet' }).click()
  await expect(html).toHaveAttribute('data-accent', 'violet')
  await expect.poll(() => primary(page)).not.toBe(blue)
  const violet = await primary(page)

  await colors.getByRole('radio', { name: 'Graphite' }).click()
  await expect(html).not.toHaveAttribute('data-accent', /.*/)
  await colors.getByRole('radio', { name: 'Violet' }).click()

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
  await expect(html).toHaveAttribute('data-accent', 'violet')
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

test('custom primary color applies, picks readable text, and persists', async ({
  page,
}) => {
  await page.goto('/')
  const html = page.locator('html')
  const panel = await openAppearance(page)
  const picker = panel.getByLabel('Custom color')
  const inline = (name: string) =>
    html.evaluate((el, prop) => el.style.getPropertyValue(prop), name)

  // A light colour takes dark text.
  await picker.fill('#ffcc33')
  await expect(html).toHaveAttribute('data-accent', 'custom')
  await expect.poll(() => inline('--primary')).toBe('#ffcc33')
  expect(await inline('--primary-foreground')).toBe('#17171c')
  await expect(
    panel
      .getByRole('radiogroup', { name: 'Primary color' })
      .getByRole('radio', { checked: true }),
  ).toHaveCount(0)

  // A dark one takes white.
  await picker.fill('#123456')
  await expect.poll(() => inline('--primary')).toBe('#123456')
  expect(await inline('--primary-foreground')).toBe('#ffffff')

  // index.html applies it before first paint.
  await page.reload()
  await expect(html).toHaveAttribute('data-accent', 'custom')
  expect(await inline('--primary')).toBe('#123456')
  expect(await inline('--primary-foreground')).toBe('#ffffff')

  // Picking a swatch drops the inline colour.
  await (
    await openAppearance(page)
  )
    .getByRole('radiogroup', { name: 'Primary color' })
    .getByRole('radio', { name: 'Rose' })
    .click()
  await expect(html).toHaveAttribute('data-accent', 'rose')
  expect(await inline('--primary')).toBe('')
  await expect(
    (await openAppearance(page)).getByLabel('Custom color'),
  ).toHaveValue('#123456')
})
