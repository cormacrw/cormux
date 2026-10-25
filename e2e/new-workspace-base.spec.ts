import { expect, test } from '@playwright/test'

test('New workspace opens at once on one-line pickers, with a searchable base branch', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Homebase' })).toBeVisible()
  await page.keyboard.press('ControlOrMeta+n')
  await expect(page.locator('#nw-prompt')).toBeFocused({ timeout: 1000 })

  // Pickers and actions share one line.
  const dialog = page.getByRole('dialog')
  const middles = await dialog
    .locator('#nw-repo, #nw-base, #nw-engine, button[type="submit"]')
    .evaluateAll((els) =>
      els.map((el) => {
        const box = el.getBoundingClientRect()
        return box.top + box.height / 2
      }),
    )
  expect(Math.max(...middles) - Math.min(...middles)).toBeLessThan(4)

  // The base branch is a search, like the workspace's branch picker.
  const base = page.locator('#nw-base')
  await expect(base).toContainText('main')
  await base.click()
  await expect(page.getByPlaceholder('Search branches…')).toBeFocused()
  await page.keyboard.type('dev')
  await expect(page.getByRole('option')).toHaveCount(1)
  await page.keyboard.press('Enter')
  await expect(base).toContainText('develop')
})
