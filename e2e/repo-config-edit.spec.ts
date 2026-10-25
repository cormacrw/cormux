import { expect, test } from '@playwright/test'

test('editing a repo config never redraws it, even as saves refresh the app', async ({
  page,
}) => {
  await page.goto('/')
  await page
    .getByRole('navigation', { name: 'Harness' })
    .getByRole('button', { name: /Settings/ })
    .click()
  await page.getByRole('button', { name: /Configure my-app/ }).click()
  const card = page.locator('[data-od-id="settings-repo-my-app"]')
  const run = card.locator('[id^="run-"]')
  await expect(card.locator('[id^="default-branch-"]')).toBeVisible()
  await run.click()
  await page.keyboard.type('p')
  await expect(card.getByText('No run command')).toHaveCount(0)

  // Count every element added or removed in the card from here on.
  await card.evaluate((el) => {
    const w = window as { __changes?: number }
    w.__changes = 0
    new MutationObserver((records) => {
      for (const record of records) {
        w.__changes! += record.addedNodes.length + record.removedNodes.length
      }
    }).observe(el, { childList: true, subtree: true })
  })
  // Slower than the save delay, so each pause saves and refreshes mid-typing.
  await run.pressSequentially('nm dev ', { delay: 450 })
  await page.waitForTimeout(600)
  expect(
    await page.evaluate(() => (window as { __changes?: number }).__changes),
  ).toBe(0)
  // The trailing space survives the saves, which store it trimmed.
  await expect(run).toHaveValue('pnm dev ')

  // Leaving the field shows the saved value.
  await page.keyboard.press('Tab')
  await expect(run).toHaveValue('pnm dev')
})
