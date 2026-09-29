import { expect, test } from '@playwright/test'

test('Appearance setting overrides the system theme and persists', async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  const html = page.locator('html')
  await expect(html).toHaveClass(/\bdark\b/)

  const openAppearance = async () => {
    await page.getByRole('button', { name: 'Settings' }).click()
    await page.getByRole('button', { name: 'General', exact: true }).click()
    return page.getByRole('group', { name: 'Appearance' })
  }

  const appearance = await openAppearance()
  await expect(appearance.getByRole('radio', { name: 'System' })).toBeChecked()
  await appearance.getByRole('radio', { name: 'Light' }).click()
  await expect(html).not.toHaveClass(/\bdark\b/)

  // index.html applies the saved choice before first paint.
  await page.reload()
  await expect(html).not.toHaveClass(/\bdark\b/)

  await (await openAppearance()).getByRole('radio', { name: 'System' }).click()
  await expect(html).toHaveClass(/\bdark\b/)
})
