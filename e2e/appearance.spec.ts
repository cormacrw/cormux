import { expect, test, type Page } from '@playwright/test'

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

test('primary color and font sizes apply and persist', async ({ page }) => {
  await page.goto('/')
  const html = page.locator('html')
  const newWorkspace = page.getByRole('button', { name: /New Workspace/ })
  const graphite = await newWorkspace.evaluate(
    (el) => getComputedStyle(el).backgroundColor,
  )

  const panel = await openAppearance(page)
  const colors = panel.getByRole('radiogroup', { name: 'Primary color' })
  await expect(colors.getByRole('radio', { name: 'Graphite' })).toBeChecked()
  await colors.getByRole('radio', { name: 'Blue' }).click()
  await expect(html).toHaveAttribute('data-accent', 'blue')

  await panel.getByLabel('UI font size').click()
  await page.getByRole('option', { name: '18px' }).click()
  await expect(html).toHaveCSS('font-size', '18px')

  await panel.getByLabel('Code font size').click()
  await page.getByRole('option', { name: '14px' }).click()

  await page.reload()
  await expect(html).toHaveAttribute('data-accent', 'blue')
  await expect(html).toHaveCSS('font-size', '18px')
  expect(
    await html.evaluate((el) =>
      getComputedStyle(el).getPropertyValue('--code-font-size').trim(),
    ),
  ).toBe('14px')
  await page.getByRole('button', { name: 'Homebase' }).click()
  await expect(newWorkspace).not.toHaveCSS('background-color', graphite)
})
