import { expect, test } from '@playwright/test'

test('copying a message swaps the icon to a check, then back', async ({
  page,
  context,
}) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  await page.goto('/?longThread=1')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const copy = page
    .locator('#timeline')
    .getByRole('button', { name: 'Copy message' })
    .first()
  await copy.hover()
  await expect(copy.locator('svg')).toHaveClass(/lucide-copy/)
  await copy.click()
  await expect(copy.locator('svg')).toHaveClass(/lucide-check/)
  expect(await page.evaluate(() => navigator.clipboard.readText())).not.toBe('')
  await expect(copy.locator('svg')).toHaveClass(/lucide-copy/, {
    timeout: 3000,
  })
})
